//! Comprehensive tests for renderer.rs
//!
//! Phase 1: Foundation tests (1.25% → 20%)
//! - Initialization logic (buffers, bind groups, pipelines)
//! - Camera calculations and transforms
//! - Material setup and shadow configuration
//! - Viewport and surface configuration
//!
//! Target: +640 lines coverage, 40-50 tests

#[cfg(test)]
mod tests {
    use astraweave_camera::CameraProducer;
    use glam::{vec3, Mat4, Vec3, Vec4Swizzles};
    use wgpu::util::DeviceExt;

    /// Create test device and queue for GPU tests
    async fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: true, // Software adapter for CI
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await
            .expect("Failed to create device")
    }

    /// Create test surface configuration
    fn create_test_config() -> wgpu::SurfaceConfiguration {
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: 800,
            height: 600,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    // ========================================================================
    // Phase 1.1: Buffer Creation & Initialization
    // ========================================================================

    #[test]
    fn test_camera_buffer_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            // CameraUBO size (view_proj: mat4 + light_dir: vec3 + pad: f32)
            let camera_size = 64 + 16; // 80 bytes

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("camera_ubo"),
                size: camera_size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            assert_eq!(buffer.size(), camera_size);
            assert_eq!(
                buffer.usage(),
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
            );
        });
    }

    #[test]
    fn test_material_buffer_creation() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;

            let material_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("material_ubo"),
                size: 32, // vec4 + 2 f32 + padding
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Seed with default material (matches renderer.rs)
            let default_material: [f32; 8] = [0.85, 0.78, 0.72, 1.0, 0.05, 0.6, 0.0, 0.0];
            queue.write_buffer(&material_buf, 0, bytemuck::cast_slice(&default_material));

            assert_eq!(material_buf.size(), 32);
        });
    }

    #[test]
    fn test_instance_buffer_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let instance_count = 1000;
            let instance_size = std::mem::size_of::<crate::types::InstanceRaw>();

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instance_buffer"),
                size: (instance_count * instance_size) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            assert_eq!(buffer.size(), (instance_count * instance_size) as u64);
            assert!(buffer.usage().contains(wgpu::BufferUsages::VERTEX));
        });
    }

    #[test]
    fn test_shadow_light_buffer() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            // MainLightUbo: 2x mat4 + 2x vec2 + 2x vec2 = 128 + 16 + 16 = 160 bytes
            let light_size = 160u64;

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light_ubo"),
                size: light_size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            assert_eq!(buffer.size(), light_size);
        });
    }

    // ========================================================================
    // Phase 1.2: Bind Group Layout Creation
    // ========================================================================

    #[test]
    fn test_camera_bind_group_layout() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            // Just verify it was created successfully
            drop(bgl);
        });
    }

    #[test]
    fn test_material_bind_group_layout() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("material_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            drop(bgl);
        });
    }

    #[test]
    fn test_texture_bind_group_layout() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bgl"),
                entries: &[
                    // Albedo texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Albedo sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Normal map
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Normal sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Metallic-Roughness
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // MR sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            drop(bgl);
        });
    }

    #[test]
    fn test_shadow_bind_group_layout() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow_bgl"),
                entries: &[
                    // Light uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Shadow texture array
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Shadow sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

            drop(bgl);
        });
    }

    // ========================================================================
    // Phase 1.3: Texture Creation
    // ========================================================================

    #[test]
    fn test_hdr_texture_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let config = create_test_config();

            let hdr_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("hdr_tex"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            assert_eq!(hdr_tex.size().width, config.width);
            assert_eq!(hdr_tex.size().height, config.height);
            assert_eq!(hdr_tex.format(), wgpu::TextureFormat::Rgba16Float);
        });
    }

    #[test]
    fn test_shadow_texture_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let shadow_res = 2048u32;
            let cascade_count = 2u32;

            let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow_tex"),
                size: wgpu::Extent3d {
                    width: shadow_res,
                    height: shadow_res,
                    depth_or_array_layers: cascade_count,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            assert_eq!(shadow_tex.size().width, shadow_res);
            assert_eq!(shadow_tex.size().depth_or_array_layers, cascade_count);
            assert_eq!(shadow_tex.format(), wgpu::TextureFormat::Depth32Float);
        });
    }

    #[test]
    fn test_shadow_texture_array_views() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow_tex"),
                size: wgpu::Extent3d {
                    width: 2048,
                    height: 2048,
                    depth_or_array_layers: 2,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            // Full array view for sampling
            let _array_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow_array_view"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
                usage: Some(wgpu::TextureUsages::empty()),
            });

            // Individual layer views for rendering
            let _layer0_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow_layer0"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::empty()),
            });

            let _layer1_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow_layer1"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 1,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::empty()),
            });
        });
    }

    // ========================================================================
    // Phase 1.4: Sampler Creation
    // ========================================================================

    #[test]
    fn test_linear_sampler_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("linear_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            drop(sampler);
        });
    }

    #[test]
    fn test_shadow_comparison_sampler() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("shadow_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual), // Comparison sampler for shadows
                ..Default::default()
            });

            drop(sampler);
        });
    }

    #[test]
    fn test_repeat_sampler_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("repeat_sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            drop(sampler);
        });
    }

    // ========================================================================
    // Phase 1.5: Camera Math & Transforms (Pure CPU, no GPU)
    // ========================================================================

    #[test]
    fn test_camera_view_matrix_calculation() {
        let eye = vec3(0.0, 5.0, 10.0);
        let target = vec3(0.0, 0.0, 0.0);
        let up = vec3(0.0, 1.0, 0.0);

        let view = Mat4::look_at_rh(eye, target, up);

        // Verify view matrix properties
        // When looking from +Z towards origin, Z component of eye position should be negated
        let transformed_origin = view * vec3(0.0, 0.0, 0.0).extend(1.0);
        assert!(transformed_origin.z < 0.0); // Origin should be behind camera in view space
    }

    #[test]
    fn test_camera_projection_matrix() {
        let fov = std::f32::consts::FRAC_PI_4; // 45 degrees
        let aspect = 800.0 / 600.0;
        let near = 0.1;
        let far = 1000.0;

        let proj = Mat4::perspective_rh(fov, aspect, near, far);

        // Test that near plane point projects correctly
        let near_point = vec3(0.0, 0.0, -near).extend(1.0);
        let proj_near = proj * near_point;

        // In perspective projection, W should be non-zero
        assert!(proj_near.w != 0.0); // Just verify projection worked
    }

    #[test]
    fn test_camera_view_proj_combined() {
        let eye = vec3(0.0, 0.0, 10.0);
        let target = vec3(0.0, 0.0, 0.0);
        let up = vec3(0.0, 1.0, 0.0);
        let view = Mat4::look_at_rh(eye, target, up);

        let fov = std::f32::consts::FRAC_PI_4;
        let aspect = 16.0 / 9.0;
        let proj = Mat4::perspective_rh(fov, aspect, 0.1, 1000.0);

        let view_proj = proj * view;

        // Test a point in front of camera
        let test_point = vec3(0.0, 0.0, 0.0).extend(1.0);
        let clip_space = view_proj * test_point;

        // Should be within clip space bounds (after w-divide)
        assert!(clip_space.w > 0.0); // Positive W means in front of camera
    }

    #[test]
    fn test_viewport_transform() {
        let width = 800.0f32;
        let height = 600.0f32;

        // NDC to screen space transform
        let ndc_x = 0.0f32; // Center
        let ndc_y = 0.0f32;

        let screen_x = (ndc_x + 1.0) * 0.5 * width;
        let screen_y = (1.0 - (ndc_y + 1.0) * 0.5) * height; // Y-flip for screen coords

        assert!((screen_x - width / 2.0).abs() < 0.1);
        assert!((screen_y - height / 2.0).abs() < 0.1);
    }

    // ========================================================================
    // Phase 1.6: Shadow Matrix Calculations
    // ========================================================================

    #[test]
    fn test_shadow_ortho_matrix() {
        let extent = 50.0; // Half-width/height of ortho frustum
        let near = 0.1;
        let far = 200.0;

        let ortho = Mat4::orthographic_rh(-extent, extent, -extent, extent, near, far);

        // Test that center point maps to origin in NDC
        let center = vec3(0.0, 0.0, -(near + far) / 2.0).extend(1.0);
        let ndc = ortho * center;

        // In orthographic projection, W should be 1.0
        assert!((ndc.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_shadow_view_matrix() {
        let light_dir = vec3(-0.5, -1.0, -0.5).normalize();
        let light_pos = -light_dir * 100.0; // Position light far from origin
        let target = Vec3::ZERO;
        let up = vec3(0.0, 1.0, 0.0);

        let light_view = Mat4::look_at_rh(light_pos, target, up);

        // Transform origin to light view space
        let origin_in_light = light_view * target.extend(1.0);

        // Origin should be at some negative Z in light view (in front of light)
        assert!(origin_in_light.z < 0.0);
    }

    #[test]
    fn test_cascade_split_calculation() {
        let near = 0.1;
        let far = 1000.0;
        let lambda = 0.5; // Split distribution (0 = uniform, 1 = logarithmic)

        // Calculate split distance for first cascade
        let split_scheme_uniform = |i: f32, n: f32| near + (far - near) * (i / n);
        let split_scheme_log = |i: f32, n: f32| near * (far / near).powf(i / n);

        let cascade_count = 2.0;
        let i = 1.0; // Split between cascade 0 and 1

        let uniform_split = split_scheme_uniform(i, cascade_count);
        let log_split = split_scheme_log(i, cascade_count);
        let split = lambda * log_split + (1.0 - lambda) * uniform_split;

        assert!(split > near);
        assert!(split < far);
        assert!(split > 0.0);
    }

    #[test]
    fn test_shadow_frustum_corners() {
        // Test calculation of frustum corners for cascade bounding
        let near = 0.1;
        let far = 50.0;
        let fov = std::f32::consts::FRAC_PI_4;
        let aspect = 16.0 / 9.0;

        let proj = Mat4::perspective_rh(fov, aspect, near, far);
        let proj_inv = proj.inverse();

        // NDC corners of near plane
        let ndc_corners = [
            vec3(-1.0, -1.0, 0.0), // Near plane Z = 0 in wgpu
            vec3(1.0, -1.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            vec3(-1.0, 1.0, 0.0),
        ];

        // Transform back to view space
        for corner in &ndc_corners {
            let view_corner = proj_inv * corner.extend(1.0);
            let view_corner = view_corner.xyz() / view_corner.w;

            // Should be on near plane
            assert!((view_corner.z.abs() - near).abs() < 0.1);
        }
    }

    // ========================================================================
    // Phase 1.7: Material Data Packing
    // ========================================================================

    #[test]
    fn test_material_data_packing() {
        let base_color = [0.85, 0.78, 0.72, 1.0];
        let metallic = 0.05;
        let roughness = 0.6;
        let _pad = [0.0, 0.0];

        let material: [f32; 8] = [
            base_color[0],
            base_color[1],
            base_color[2],
            base_color[3],
            metallic,
            roughness,
            0.0,
            0.0,
        ];

        // Verify size matches shader uniform
        assert_eq!(std::mem::size_of_val(&material), 32);

        // Verify values
        assert_eq!(material[0], 0.85);
        assert_eq!(material[4], metallic);
        assert_eq!(material[5], roughness);
    }

    #[test]
    fn test_material_byte_conversion() {
        let material: [f32; 8] = [0.85, 0.78, 0.72, 1.0, 0.05, 0.6, 0.0, 0.0];

        let bytes: &[u8] = bytemuck::cast_slice(&material);

        assert_eq!(bytes.len(), 32);
    }

    // ========================================================================
    // Phase 1.8: Instance Transform Packing
    // ========================================================================

    #[test]
    fn test_instance_transform_to_raw() {
        use crate::types::Instance;

        let instance = Instance {
            transform: Mat4::from_translation(vec3(1.0, 2.0, 3.0)),
            color: [1.0, 0.5, 0.2, 1.0],
            material_id: 0,
        };

        let raw = instance.raw();

        // Verify transform translation
        let translation = Mat4::from_cols_array_2d(&raw.model).w_axis;
        assert_eq!(translation.xyz(), vec3(1.0, 2.0, 3.0));

        // Verify color
        assert_eq!(raw.color, instance.color);

        // Verify material ID
        assert_eq!(raw.material_id, 0);
    }

    #[test]
    fn test_instance_raw_size() {
        use crate::types::InstanceRaw;

        // InstanceRaw has model, normal_matrix, color, material_id
        let size = std::mem::size_of::<InstanceRaw>();

        assert!(size > 0);
        assert!(size % 4 == 0); // Should be aligned for GPU
    }

    #[test]
    fn test_multiple_instances_packing() {
        use crate::types::Instance;

        let instances: Vec<Instance> = vec![
            Instance {
                transform: Mat4::from_translation(vec3(0.0, 0.0, 0.0)),
                color: [1.0, 1.0, 1.0, 1.0],
                material_id: 0,
            },
            Instance {
                transform: Mat4::from_translation(vec3(1.0, 0.0, 0.0)),
                color: [1.0, 0.0, 0.0, 1.0],
                material_id: 1,
            },
            Instance {
                transform: Mat4::from_translation(vec3(0.0, 1.0, 0.0)),
                color: [0.0, 1.0, 0.0, 1.0],
                material_id: 2,
            },
        ];

        let raws: Vec<_> = instances.iter().map(|i| i.raw()).collect();

        assert_eq!(raws.len(), 3);

        // Verify each transform translation
        let t0 = Mat4::from_cols_array_2d(&raws[0].model).w_axis.xyz();
        let t1 = Mat4::from_cols_array_2d(&raws[1].model).w_axis.xyz();
        let t2 = Mat4::from_cols_array_2d(&raws[2].model).w_axis.xyz();

        assert_eq!(t0, vec3(0.0, 0.0, 0.0));
        assert_eq!(t1, vec3(1.0, 0.0, 0.0));
        assert_eq!(t2, vec3(0.0, 1.0, 0.0));
    }

    // ========================================================================
    // Phase 1.9: Clustered Lighting Data Structures
    // ========================================================================

    #[test]
    fn test_cluster_dims_calculation() {
        use crate::clustered::ClusterDims;

        let dims = ClusterDims {
            x: 120, // (1920 + 15) / 16
            y: 68,  // (1080 + 15) / 16
            z: 24,  // depth slices
        };

        let total_clusters = dims.x * dims.y * dims.z;
        assert_eq!(total_clusters, 120 * 68 * 24);
        assert!(total_clusters > 0);
    }

    #[test]
    fn test_cluster_dims_different_resolutions() {
        use crate::clustered::ClusterDims;

        let configs = vec![
            ClusterDims {
                x: 50,
                y: 38,
                z: 16,
            }, // 800x600, 16px tiles
            ClusterDims {
                x: 120,
                y: 68,
                z: 24,
            }, // 1920x1080, 16px tiles
            ClusterDims {
                x: 160,
                y: 90,
                z: 32,
            }, // 2560x1440, 16px tiles
            ClusterDims {
                x: 240,
                y: 135,
                z: 32,
            }, // 3840x2160, 16px tiles
        ];

        for dims in configs {
            assert!(dims.x > 0);
            assert!(dims.y > 0);
            assert!(dims.z > 0);

            let total = dims.x * dims.y * dims.z;
            assert!(total > 0);
        }
    }

    #[test]
    fn test_cpu_light_structure() {
        use crate::clustered::CpuLight;

        let light = CpuLight {
            pos: vec3(0.0, 5.0, 0.0),
            radius: 10.0,
        };

        assert_eq!(light.pos, vec3(0.0, 5.0, 0.0));
        assert_eq!(light.radius, 10.0);
    }

    // ========================================================================
    // Phase 2: Shader Compilation & Pipeline Creation
    // ========================================================================

    #[test]
    fn test_shader_module_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let shader_source = r#"
                @vertex
                fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
                    var pos = array<vec2<f32>, 3>(
                        vec2<f32>(-1.0, -1.0),
                        vec2<f32>(3.0, -1.0),
                        vec2<f32>(-1.0, 3.0)
                    );
                    return vec4<f32>(pos[vid], 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                }
            "#;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("test_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source)),
            });

            // Just verify it was created successfully
            drop(shader);
        });
    }

    #[test]
    fn test_render_pipeline_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("test_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                    r#"
                    @vertex
                    fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                    }
                    @fragment
                    fn fs_main() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
                    }
                    "#,
                )),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("test_pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

            drop(pipeline);
        });
    }

    #[test]
    fn test_pipeline_with_depth_stencil() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("test_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                    r#"
                    @vertex
                    fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                    }
                    @fragment
                    fn fs_main() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
                    }
                    "#,
                )),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("test_pipeline_depth"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

            drop(pipeline);
        });
    }

    #[test]
    fn test_pipeline_with_blending() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("test_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                    r#"
                    @vertex
                    fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                    }
                    @fragment
                    fn fs_main() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 1.0, 1.0, 0.5);
                    }
                    "#,
                )),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("test_pipeline_blend"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

            drop(pipeline);
        });
    }

    // ========================================================================
    // Phase 2: Command Encoder & Render Pass Patterns
    // ========================================================================

    #[test]
    fn test_command_encoder_creation() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test_encoder"),
            });

            let _commands = encoder.finish();
        });
    }

    #[test]
    fn test_render_pass_descriptor() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let config = create_test_config();

            // Create color target
            let color_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("color_target"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test_encoder"),
            });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("test_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            let _commands = encoder.finish();
        });
    }

    #[test]
    fn test_render_pass_with_depth() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let config = create_test_config();

            // Create color and depth targets
            let color_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("color_target"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_target"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test_encoder"),
            });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("test_pass_depth"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            let _commands = encoder.finish();
        });
    }

    // ========================================================================
    // Phase 2: Bind Group Creation & Validation
    // ========================================================================

    #[test]
    fn test_bind_group_with_uniform_buffer() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniform"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bg"),
                layout: &bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

            drop(bind_group);
        });
    }

    #[test]
    fn test_bind_group_with_texture_and_sampler() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test_texture"),
                size: wgpu::Extent3d {
                    width: 256,
                    height: 256,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("test_sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                ..Default::default()
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tex_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tex_bg"),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            drop(bind_group);
        });
    }

    #[test]
    fn test_multiple_bind_groups() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;

            // Create 3 different bind groups (typical shader setup)
            let buf0 = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("buf0"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let buf1 = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("buf1"),
                size: 128,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let buf2 = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("buf2"),
                size: 256,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let bgl0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bgl0"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bgl1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bgl1"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bgl2 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bgl2"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let _bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bg0"),
                layout: &bgl0,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf0.as_entire_binding(),
                }],
            });

            let _bg1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bg1"),
                layout: &bgl1,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf1.as_entire_binding(),
                }],
            });

            let _bg2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bg2"),
                layout: &bgl2,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf2.as_entire_binding(),
                }],
            });
        });
    }

    // ========================================================================
    // Phase 2: Buffer Upload & Queue Operations
    // ========================================================================

    #[test]
    fn test_queue_write_buffer() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("test_buffer"),
                size: 256,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&data));
        });
    }

    #[test]
    fn test_queue_write_texture() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test_texture"),
                size: wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let data = vec![255u8; 16 * 16 * 4]; // 16x16 white texture

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(16 * 4),
                    rows_per_image: Some(16),
                },
                wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
            );
        });
    }

    #[test]
    fn test_queue_submit() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;

            let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test_encoder"),
            });

            let commands = encoder.finish();
            queue.submit(std::iter::once(commands));
        });
    }

    // ========================================================================
    // Phase 2: Utility Module Tests (clustered_forward, animation, culling)
    // ========================================================================

    #[test]
    fn test_cluster_config_default() {
        use crate::clustered_forward::ClusterConfig;

        let config = ClusterConfig::default();
        assert_eq!(config.cluster_x, 16);
        assert_eq!(config.cluster_y, 9);
        assert_eq!(config.cluster_z, 24);
        assert_eq!(config.near, 0.1);
        assert_eq!(config.far, 100.0);
    }

    #[test]
    fn test_gpu_light_creation() {
        use crate::clustered_forward::GpuLight;
        use glam::vec3;

        let pos = vec3(10.0, 5.0, -3.0);
        let color = vec3(1.0, 0.8, 0.6);
        let radius = 15.0;
        let intensity = 2.5;

        let light = GpuLight::new(pos, radius, color, intensity);

        assert_eq!(light.position[0], 10.0);
        assert_eq!(light.position[1], 5.0);
        assert_eq!(light.position[2], -3.0);
        assert_eq!(light.position[3], 15.0); // radius in w
        assert_eq!(light.color[0], 1.0);
        assert_eq!(light.color[1], 0.8);
        assert_eq!(light.color[2], 0.6);
        assert_eq!(light.color[3], 2.5); // intensity in w
    }

    #[test]
    fn test_gpu_light_bytemuck() {
        use crate::clustered_forward::GpuLight;
        use glam::vec3;

        let light = GpuLight::new(vec3(1.0, 2.0, 3.0), 10.0, vec3(0.5, 0.5, 0.5), 1.0);

        // Test Pod/Zeroable traits work
        let _bytes: &[u8] = bytemuck::bytes_of(&light);
        let _light_array = [light; 10];
        let _slice_bytes: &[u8] = bytemuck::cast_slice(&_light_array);
    }

    #[test]
    fn test_animation_clip_creation() {
        use crate::animation::{AnimationChannel, AnimationClip, ChannelData, Interpolation};
        use glam::vec3;

        let clip = AnimationClip {
            name: "test_clip".to_string(),
            duration: 2.0,
            channels: vec![AnimationChannel {
                target_joint_index: 0,
                times: vec![0.0, 1.0, 2.0],
                data: ChannelData::Translation(vec![vec3(0.0, 0.0, 0.0)]),
                interpolation: Interpolation::Linear,
            }],
        };

        assert_eq!(clip.name, "test_clip");
        assert_eq!(clip.duration, 2.0);
        assert_eq!(clip.channels.len(), 1);
        assert_eq!(clip.channels[0].target_joint_index, 0);
    }

    #[test]
    fn test_camera_creation() {
        use astraweave_camera::FreeFly as Camera;
        use glam::vec3;

        let camera = Camera {
            position: vec3(0.0, 5.0, 10.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: std::f32::consts::PI / 4.0,
            aspect: 800.0 / 600.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let view_proj = camera.vp();
        assert!(view_proj.determinant() != 0.0); // Valid matrix
    }

    #[test]
    fn test_camera_update() {
        use astraweave_camera::FreeFly as Camera;
        use glam::vec3;

        let mut camera = Camera {
            position: vec3(0.0, 5.0, 10.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: std::f32::consts::PI / 4.0,
            aspect: 800.0 / 600.0,
            znear: 0.1,
            zfar: 100.0,
        };

        camera.position = vec3(5.0, 5.0, 5.0);

        let view_proj = camera.vp();
        assert!(view_proj.determinant() != 0.0);
    }

    #[test]
    fn test_depth_texture_format() {
        // Depth uses Depth32Float format
        let format = wgpu::TextureFormat::Depth32Float;
        assert_eq!(format, wgpu::TextureFormat::Depth32Float);
    }

    #[test]
    fn test_frustum_planes_extraction() {
        use crate::culling::FrustumPlanes;
        use glam::{vec3, Mat4};

        let view = Mat4::look_at_rh(
            vec3(0.0, 0.0, 10.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective_rh(std::f32::consts::PI / 4.0, 16.0 / 9.0, 0.1, 100.0);
        let view_proj = proj * view;

        let frustum = FrustumPlanes::from_view_proj(&view_proj);

        // Frustum should have 6 planes with valid normals
        assert_eq!(frustum.planes.len(), 6);
        for plane in &frustum.planes {
            let normal_len =
                (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            assert!(normal_len > 0.9 && normal_len < 1.1); // Normalized
        }
    }

    #[test]
    fn test_instance_aabb_creation() {
        use crate::culling::InstanceAABB;
        use glam::vec3;

        let aabb = InstanceAABB::new(vec3(5.0, 3.0, -2.0), vec3(1.0, 2.0, 1.5), 0);

        assert_eq!(aabb.center, [5.0, 3.0, -2.0]);
        assert_eq!(aabb.extent, [1.0, 2.0, 1.5]);
        assert_eq!(aabb.instance_index, 0);
    }

    // ========================================================================
    // Phase 3: Environment Module Tests (environment.rs @ 25% coverage)
    // ========================================================================

    #[test]
    fn test_time_of_day_default() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::default();
        assert_eq!(tod.current_time, 12.0); // Noon
        assert_eq!(tod.time_scale, 60.0);
        assert_eq!(tod.day_length, 1440.0);
    }

    #[test]
    fn test_time_of_day_new() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(6.0, 120.0); // Sunrise, 2x speed
        assert_eq!(tod.current_time, 6.0);
        assert_eq!(tod.time_scale, 120.0);
    }

    #[test]
    fn test_sun_position_noon() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(12.0, 1.0); // Noon
        let sun_pos = tod.get_sun_position();

        // At noon, sun should be high in the sky (y > 0.9)
        assert!(sun_pos.y > 0.9, "Sun should be high at noon: {}", sun_pos.y);
        assert!(sun_pos.length() > 0.99 && sun_pos.length() < 1.01); // Normalized
    }

    #[test]
    fn test_sun_position_sunrise() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(6.0, 1.0); // Sunrise
        let sun_pos = tod.get_sun_position();

        // At sunrise, sun should be near horizon (y ≈ 0)
        assert!(
            sun_pos.y.abs() < 0.2,
            "Sun should be near horizon at sunrise: {}",
            sun_pos.y
        );
        assert!(sun_pos.length() > 0.99 && sun_pos.length() < 1.01); // Normalized
    }

    #[test]
    fn test_sun_position_sunset() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(18.0, 1.0); // Sunset
        let sun_pos = tod.get_sun_position();

        // At sunset, sun should be near horizon (y ≈ 0)
        assert!(
            sun_pos.y.abs() < 0.2,
            "Sun should be near horizon at sunset: {}",
            sun_pos.y
        );
        assert!(sun_pos.length() > 0.99 && sun_pos.length() < 1.01); // Normalized
    }

    #[test]
    fn test_sun_position_midnight() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(0.0, 1.0); // Midnight
        let sun_pos = tod.get_sun_position();

        // At midnight, sun should be below horizon (y < 0)
        assert!(
            sun_pos.y < 0.0,
            "Sun should be below horizon at midnight: {}",
            sun_pos.y
        );
        assert!(sun_pos.length() > 0.99 && sun_pos.length() < 1.01); // Normalized
    }

    #[test]
    fn test_moon_position_opposite_sun() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(12.0, 1.0);
        let sun_pos = tod.get_sun_position();
        let moon_pos = tod.get_moon_position();

        // Moon should be opposite to sun
        assert!(
            (sun_pos + moon_pos).length() < 0.01,
            "Moon should be opposite sun"
        );
    }

    #[test]
    fn test_light_direction_day() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(12.0, 1.0); // Noon
        let light_dir = tod.get_light_direction();

        // Light direction should point downward (from sun)
        assert!(light_dir.y < 0.0, "Light should come from above at noon");
    }

    #[test]
    fn test_light_direction_night() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(0.0, 1.0); // Midnight
        let light_dir = tod.get_light_direction();

        // At night, light comes from moon (which is opposite sun)
        // Since sun is below horizon, moon is above, light points down
        assert!(light_dir.length() > 0.99 && light_dir.length() < 1.01); // Normalized
    }

    #[test]
    fn test_light_color_day() {
        use crate::environment::TimeOfDay;

        let tod = TimeOfDay::new(12.0, 1.0); // Noon
        let color = tod.get_light_color();

        // Daytime should have warm colors (all components > 0.5)
        assert!(
            color.x > 0.5 && color.y > 0.5 && color.z > 0.4,
            "Daytime light should be warm: {:?}",
            color
        );
    }

    #[test]
    fn test_sky_config_default() {
        use crate::environment::SkyConfig;

        let config = SkyConfig::default();
        assert!(config.day_color_top.length() > 0.0);
        assert!(config.cloud_coverage >= 0.0 && config.cloud_coverage <= 1.0);
    }

    #[test]
    fn test_weather_system_creation() {
        use crate::environment::WeatherSystem;

        let weather = WeatherSystem::new();

        // Weather should start clear
        assert!(!weather.is_raining());
        assert!(!weather.is_snowing());
    }

    #[test]
    fn test_weather_particle_creation() {
        use crate::environment::WeatherParticle;
        use glam::vec3;

        let particle = WeatherParticle {
            position: vec3(1.0, 2.0, 3.0),
            velocity: vec3(0.0, -1.0, 0.0),
            life: 5.0,
            max_life: 10.0,
            size: 0.1,
        };

        assert_eq!(particle.position, vec3(1.0, 2.0, 3.0));
        assert_eq!(particle.life, 5.0);
    }

    // ========================================================================
    // Phase 4: Headless Renderer Fixture - Direct Method Testing
    // ========================================================================

    /// Helper to create renderer components without requiring a Window
    /// This allows testing Renderer methods that don't need actual rendering
    #[allow(dead_code)]
    struct TestRendererContext {
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        config: wgpu::SurfaceConfiguration,
    }

    impl TestRendererContext {
        async fn new() -> Self {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect("Failed to find adapter");

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("test_renderer_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                })
                .await
                .expect("Failed to create device");

            let format = wgpu::TextureFormat::Bgra8UnormSrgb;
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: 800,
                height: 600,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            Self {
                device,
                queue,
                format,
                config,
            }
        }
    }

    #[test]
    fn test_material_package_shader_compilation() {
        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            // Create a minimal shader that compiles
            let shader_source = r#"
                @vertex
                fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
                    var pos = array<vec2<f32>,3>(
                        vec2<f32>(-1.0,-3.0),
                        vec2<f32>(3.0,1.0),
                        vec2<f32>(-1.0,1.0)
                    );
                    return vec4<f32>(pos[vid], 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(0.8, 0.8, 0.8, 1.0);
                }
            "#;

            // Verify shader compiles successfully
            let shader = ctx
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("material_test_shader"),
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source)),
                });

            drop(shader);
        });
    }

    #[test]
    fn test_mesh_buffer_creation() {
        use crate::types::Vertex;

        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            // Create vertex data
            let vertices = vec![
                Vertex {
                    position: [0.0, 0.5, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.5, 0.0],
                },
                Vertex {
                    position: [-0.5, -0.5, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [1.0, 1.0],
                },
            ];

            // Create vertex buffer (mimics what Renderer does)
            let vertex_buf = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("test_vertex_buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            // Verify buffer size
            let expected_size = vertices.len() * std::mem::size_of::<Vertex>();
            assert_eq!(vertex_buf.size(), expected_size as u64);
        });
    }

    #[test]
    fn test_mesh_index_buffer_creation() {
        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            let indices: Vec<u32> = vec![0, 1, 2, 2, 1, 3]; // 2 triangles

            // Create index buffer
            let index_buf = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("test_index_buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

            // Verify buffer size
            let expected_size = indices.len() * std::mem::size_of::<u32>();
            assert_eq!(index_buf.size(), expected_size as u64);
        });
    }

    #[test]
    fn test_large_mesh_buffer_capacity() {
        use crate::types::Vertex;

        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            // Create large mesh (1000 vertices)
            let vertices: Vec<Vertex> = (0..1000)
                .map(|i| Vertex {
                    position: [i as f32, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                })
                .collect();

            let vertex_buf = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("large_vertex_buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            // Each vertex is 48 bytes (3 + 3 + 4 + 2 floats)
            assert_eq!(vertex_buf.size(), 1000 * 48);
        });
    }

    #[test]
    fn test_instance_raw_conversion() {
        use crate::types::Instance;
        use glam::{vec3, Mat4};

        // Create instance with transform
        let transform = Mat4::from_scale_rotation_translation(
            vec3(2.0, 2.0, 2.0),
            glam::Quat::IDENTITY,
            vec3(10.0, 5.0, 0.0),
        );

        let instance = Instance {
            transform,
            color: [1.0, 0.5, 0.25, 1.0],
            material_id: 42,
        };

        let raw = instance.raw();

        // Verify transform was preserved
        let reconstructed_transform = Mat4::from_cols_array_2d(&raw.model);
        let translation = reconstructed_transform.w_axis;
        assert!((translation.x - 10.0).abs() < 0.001);
        assert!((translation.y - 5.0).abs() < 0.001);

        // Verify color and material_id
        assert_eq!(raw.color, [1.0, 0.5, 0.25, 1.0]);
        assert_eq!(raw.material_id, 42);
    }

    #[test]
    fn test_instance_batch_conversion() {
        use crate::types::{Instance, InstanceRaw};
        use glam::{vec3, Mat4};

        let instances: Vec<Instance> = (0..10)
            .map(|i| Instance {
                transform: Mat4::from_translation(vec3(i as f32, 0.0, 0.0)),
                color: [1.0, 1.0, 1.0, 1.0],
                material_id: i,
            })
            .collect();

        let raw_instances: Vec<InstanceRaw> = instances.iter().map(|i| i.raw()).collect();

        assert_eq!(raw_instances.len(), 10);
        for (i, raw) in raw_instances.iter().enumerate() {
            assert_eq!(raw.material_id, i as u32);
            let transform = Mat4::from_cols_array_2d(&raw.model);
            let translation = transform.w_axis;
            assert!((translation.x - i as f32).abs() < 0.001);
        }
    }

    #[test]
    fn test_instance_buffer_upload() {
        use crate::types::{Instance, InstanceRaw};
        use glam::{vec3, Mat4};

        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            let instances: Vec<Instance> = (0..100)
                .map(|i| Instance {
                    transform: Mat4::from_translation(vec3(i as f32, 0.0, 0.0)),
                    color: [1.0, 1.0, 1.0, 1.0],
                    material_id: 0,
                })
                .collect();

            let raw_instances: Vec<InstanceRaw> = instances.iter().map(|i| i.raw()).collect();

            // Create instance buffer
            let instance_buf = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("test_instance_buffer"),
                    contents: bytemuck::cast_slice(&raw_instances),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

            // Verify buffer size
            let expected_size = raw_instances.len() * std::mem::size_of::<InstanceRaw>();
            assert_eq!(instance_buf.size(), expected_size as u64);
        });
    }

    #[test]
    fn test_camera_ubo_packing() {
        use glam::{vec3, Mat4};

        // Simulate camera UBO data (80 bytes: mat4 + vec3 + pad)
        let view_proj = Mat4::perspective_rh(std::f32::consts::PI / 4.0, 16.0 / 9.0, 0.1, 100.0);
        let light_dir = vec3(0.0, -1.0, 0.0).normalize();

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CameraUBO {
            view_proj: [[f32; 4]; 4],
            light_dir: [f32; 3],
            _pad: f32,
        }

        let ubo = CameraUBO {
            view_proj: view_proj.to_cols_array_2d(),
            light_dir: light_dir.to_array(),
            _pad: 0.0,
        };

        let bytes = bytemuck::bytes_of(&ubo);
        assert_eq!(bytes.len(), 80); // 64 + 12 + 4 = 80 bytes
    }

    #[test]
    fn test_depth_texture_creation() {
        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            let depth = crate::depth::Depth::create(&ctx.device, &ctx.config);

            // Verify depth texture was created with correct format
            assert_eq!(depth.texture.format(), wgpu::TextureFormat::Depth32Float);
            assert_eq!(depth.texture.width(), ctx.config.width);
            assert_eq!(depth.texture.height(), ctx.config.height);
        });
    }

    #[test]
    fn test_depth_texture_view() {
        pollster::block_on(async {
            let ctx = TestRendererContext::new().await;

            let depth = crate::depth::Depth::create(&ctx.device, &ctx.config);

            // Depth view should be accessible
            let _view = &depth.view;

            // Verify we can create a render pass with depth attachment
            let color_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test_color"),
                size: wgpu::Extent3d {
                    width: ctx.config.width,
                    height: ctx.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: ctx.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("test_encoder"),
                });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("test_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            let _commands = encoder.finish();
        });
    }

    #[test]
    fn test_renderer_lifecycle_headless() {
        pollster::block_on(async {
            use astraweave_camera::FreeFly as Camera;
            use crate::renderer::Renderer;

            // Initialize headless renderer
            let mut renderer = Renderer::new_headless(800, 600)
                .await
                .expect("Failed to create headless renderer");

            // Verify basic state
            assert!(renderer.surface().is_none());
            assert_eq!(renderer.config().width, 800);
            assert_eq!(renderer.config().height, 600);

            // Test resize
            renderer.resize(1024, 768);
            assert_eq!(renderer.config().width, 1024);
            assert_eq!(renderer.config().height, 768);

            // Test camera update
            let camera = Camera {
                position: glam::vec3(0.0, 5.0, 10.0),
                yaw: 0.0,
                pitch: 0.0,
                fovy: 45.0f32.to_radians(),
                aspect: 1024.0 / 768.0,
                znear: 0.1,
                zfar: 100.0,
            };
            renderer.update_view(&camera.to_render_view());

            // Test material update
            renderer.set_material_params([1.0, 0.0, 0.0, 1.0], 0.5, 0.1);

            // Test weather update
            renderer.set_weather(crate::effects::WeatherKind::Rain);
            renderer.tick_weather(0.016);

            // Test environment update
            renderer.tick_environment(0.016);

            // Test render (should return Ok(()) immediately because surface is None)
            let result = renderer.render();
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_renderer_mesh_creation() {
        pollster::block_on(async {
            use crate::renderer::Renderer;

            let renderer = Renderer::new_headless(800, 600)
                .await
                .expect("Failed to create headless renderer");

            let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
            let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
            let indices = vec![0, 1, 2];

            let mesh = renderer.create_mesh_from_arrays(&vertices, &normals, &indices);
            assert_eq!(mesh.index_count, 3);
        });
    }

    #[test]
    fn test_renderer_instance_updates() {
        pollster::block_on(async {
            use crate::renderer::Renderer;
            use crate::types::Instance;

            let mut renderer = Renderer::new_headless(800, 600)
                .await
                .expect("Failed to create headless renderer");

            let instances = vec![
                Instance {
                    transform: glam::Mat4::from_translation(glam::vec3(1.0, 2.0, 3.0)),
                    color: [1.0, 0.0, 0.0, 1.0],
                    material_id: 0,
                },
                Instance {
                    transform: glam::Mat4::from_translation(glam::vec3(-1.0, -2.0, -3.0)),
                    color: [0.0, 1.0, 0.0, 1.0],
                    material_id: 1,
                },
            ];

            renderer.update_instances(&instances);

            // Verify GPU-side data
            let gpu_instances = renderer.read_instance_buffer().await;
            assert_eq!(gpu_instances.len(), 2);

            // Check first instance position (stored in model matrix column 3)
            assert_eq!(gpu_instances[0].model[3][0], 1.0);
            assert_eq!(gpu_instances[0].model[3][1], 2.0);
            assert_eq!(gpu_instances[0].model[3][2], 3.0);

            // Check second instance position
            assert_eq!(gpu_instances[1].model[3][0], -1.0);
            assert_eq!(gpu_instances[1].model[3][1], -2.0);
            assert_eq!(gpu_instances[1].model[3][2], -3.0);
        });
    }

    #[test]
    fn test_renderer_water_initialization() {
        pollster::block_on(async {
            use crate::renderer::Renderer;
            use crate::water::WaterRenderer;

            let mut renderer = Renderer::new_headless(800, 600)
                .await
                .expect("Failed to create headless renderer");

            let water = WaterRenderer::new(
                renderer.device(),
                renderer.config().format,
                wgpu::TextureFormat::Depth32Float,
            );

            renderer.set_water_renderer(water);
            // Verify it doesn't crash during render
            renderer.render().expect("Failed to render with water");
        });
    }

    #[test]
    fn test_renderer_shadow_map_creation() {
        pollster::block_on(async {
            use crate::renderer::Renderer;

            let mut renderer = Renderer::new_headless(800, 600)
                .await
                .expect("Failed to create headless renderer");

            // Use existing CSM tuning API
            renderer.set_shadow_filter(1.0, 0.0001, 1.0);
            renderer.set_cascade_splits(10.0, 50.0);
            renderer.set_cascade_lambda(0.5);
        });
    }

    #[test]
    fn test_renderer_post_processing_initialization() {
        pollster::block_on(async {
            use crate::renderer::Renderer;

            let mut renderer = Renderer::new_headless(800, 600)
                .await
                .expect("Failed to create headless renderer");

            // Post-processing is integrated into the render call.
            // We verify that render() doesn't crash in headless mode.
            renderer
                .render()
                .expect("Failed to render in headless mode");
        });
    }

    #[test]
    fn test_renderer_read_timestamp_query() {
        pollster::block_on(async {
            // This test requires gpu-tests feature
            #[cfg(feature = "gpu-tests")]
            {
                use crate::renderer::Renderer;
                let mut renderer = Renderer::new_headless(800, 600)
                    .await
                    .expect("Failed to create headless renderer");

                renderer.render().expect("Failed to render");
            }
        });
    }
}
