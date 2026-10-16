//! Render Pipeline GPU Tests
//!
//! Tests for render-specific GPU functionality:
//! - PBR material pipeline
//! - Camera uniform binding
//! - Light uniform binding
//! - Shadow cascades
//! - Mesh vertex formats
//! - Draw call validation

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use wgpu::util::DeviceExt;

// =============================================================================
// Test Infrastructure
// =============================================================================

async fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    gpu_capability::request_headless(
        "render_pipeline_test_device",
        wgpu::Features::empty(),
        wgpu::Limits::downlevel_defaults(),
        wgpu::PowerPreference::default(),
        true,
    )
    .await
    .map(|gpu| (gpu.device, gpu.queue))
}

// =============================================================================
// Camera Uniform Tests
// =============================================================================

mod camera_tests {
    use super::*;

    /// Camera uniform buffer layout (matches CameraUBO in renderer.rs)
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct CameraUbo {
        view: [[f32; 4]; 4],
        proj: [[f32; 4]; 4],
        view_proj: [[f32; 4]; 4],
        inv_view: [[f32; 4]; 4],
        inv_proj: [[f32; 4]; 4],
        eye_pos: [f32; 4],
    }

    #[test]
    fn test_camera_ubo_size() {
        assert_eq!(
            std::mem::size_of::<CameraUbo>(),
            336, // 5 * 64 bytes (4x4 matrices) + 16 bytes (vec4)
            "CameraUbo should be 336 bytes"
        );
    }

    #[test]
    fn test_camera_uniform_buffer_creation() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let camera = CameraUbo {
                view: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, -5.0, 1.0],
                ],
                proj: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                view_proj: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                inv_view: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                inv_proj: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                eye_pos: [0.0, 0.0, 5.0, 1.0],
            };

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("camera_ubo"),
                contents: bytemuck::bytes_of(&camera),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            assert_eq!(buffer.size(), 336);
        });
    }

    #[test]
    fn test_camera_bind_group_layout() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(336),
                    },
                    count: None,
                }],
            });

            // Create buffer and bind group
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("camera_buffer"),
                size: 336,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let _bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("camera_bind_group"),
                layout: &bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
        });
    }
}

// =============================================================================
// Light Uniform Tests
// =============================================================================

mod light_tests {
    use super::*;

    /// Main light uniform buffer layout (matches MainLightUbo in renderer.rs)
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct MainLightUbo {
        direction: [f32; 4],
        color: [f32; 4],
        intensity: f32,
        _padding: [f32; 3],
    }

    #[test]
    fn test_light_ubo_size() {
        assert_eq!(
            std::mem::size_of::<MainLightUbo>(),
            48,
            "MainLightUbo should be 48 bytes"
        );
    }

    #[test]
    fn test_light_uniform_buffer() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            let light = MainLightUbo {
                direction: [0.0, -1.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                intensity: 1.0,
                _padding: [0.0; 3],
            };

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("light_ubo"),
                contents: bytemuck::bytes_of(&light),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // Update light at runtime
            let updated_light = MainLightUbo {
                direction: [1.0, -1.0, 0.0, 0.0],
                color: [1.0, 0.9, 0.8, 1.0],
                intensity: 1.5,
                _padding: [0.0; 3],
            };

            queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&updated_light));
        });
    }

    #[test]
    fn test_light_bind_group_layout() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("light_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(48),
                    },
                    count: None,
                }],
            });

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light_buffer"),
                size: 48,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let _bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("light_bind_group"),
                layout: &bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
        });
    }
}

// =============================================================================
// Material Uniform Tests
// =============================================================================

mod material_tests {
    use super::*;

    /// Material uniform buffer layout (matches MaterialUbo in renderer.rs)
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct MaterialUbo {
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
        ao: f32,
        _padding: f32,
    }

    #[test]
    fn test_material_ubo_size() {
        assert_eq!(
            std::mem::size_of::<MaterialUbo>(),
            32,
            "MaterialUbo should be 32 bytes"
        );
    }

    #[test]
    fn test_material_buffer_creation() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let material = MaterialUbo {
                base_color: [0.8, 0.2, 0.1, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                ao: 1.0,
                _padding: 0.0,
            };

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material_ubo"),
                contents: bytemuck::bytes_of(&material),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            assert_eq!(buffer.size(), 32);
        });
    }

    #[test]
    fn test_metallic_material() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let metal = MaterialUbo {
                base_color: [0.9, 0.85, 0.8, 1.0], // Gold-ish
                metallic: 1.0,
                roughness: 0.1,
                ao: 1.0,
                _padding: 0.0,
            };

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("metal_material"),
                contents: bytemuck::bytes_of(&metal),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            assert!(buffer.size() >= 32);
        });
    }

    #[test]
    fn test_dielectric_material() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let plastic = MaterialUbo {
                base_color: [0.1, 0.4, 0.8, 1.0], // Blue plastic
                metallic: 0.0,
                roughness: 0.4,
                ao: 1.0,
                _padding: 0.0,
            };

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("plastic_material"),
                contents: bytemuck::bytes_of(&plastic),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            assert!(buffer.size() >= 32);
        });
    }
}

// =============================================================================
// Vertex Format Tests
// =============================================================================

mod vertex_tests {
    use super::*;

    /// Standard PBR vertex format
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct PbrVertex {
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        tangent: [f32; 4],
    }

    #[test]
    fn test_vertex_size() {
        assert_eq!(
            std::mem::size_of::<PbrVertex>(),
            48,
            "PbrVertex should be 48 bytes"
        );
    }

    #[test]
    fn test_vertex_buffer_creation() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            // Triangle vertices
            let vertices = [
                PbrVertex {
                    position: [0.0, 0.5, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.5, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                },
                PbrVertex {
                    position: [-0.5, -0.5, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                },
                PbrVertex {
                    position: [0.5, -0.5, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [1.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                },
            ];

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            assert_eq!(buffer.size(), 144); // 3 * 48
        });
    }

    #[test]
    fn test_index_buffer_creation() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            // Quad indices (two triangles)
            let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            assert_eq!(buffer.size(), 12); // 6 * 2 bytes
        });
    }

    #[test]
    fn test_vertex_buffer_layout() {
        let layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PbrVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Normal
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
                // UV
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 2,
                },
                // Tangent
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 3,
                },
            ],
        };

        assert_eq!(layout.array_stride, 48);
        assert_eq!(layout.attributes.len(), 4);
    }
}

// =============================================================================
// Shadow Cascade Tests
// =============================================================================

mod shadow_tests {
    use super::*;

    const CASCADE_COUNT: u32 = 4;
    const SHADOW_MAP_SIZE: u32 = 2048;

    #[test]
    fn test_shadow_map_texture_array() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let shadow_map = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow_cascades"),
                size: wgpu::Extent3d {
                    width: SHADOW_MAP_SIZE,
                    height: SHADOW_MAP_SIZE,
                    depth_or_array_layers: CASCADE_COUNT,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            assert_eq!(shadow_map.size().width, SHADOW_MAP_SIZE);
            assert_eq!(shadow_map.size().depth_or_array_layers, CASCADE_COUNT);
        });
    }

    #[test]
    fn test_shadow_cascade_views() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let shadow_map = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow_cascades"),
                size: wgpu::Extent3d {
                    width: SHADOW_MAP_SIZE,
                    height: SHADOW_MAP_SIZE,
                    depth_or_array_layers: CASCADE_COUNT,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            // Create view for each cascade
            for i in 0..CASCADE_COUNT {
                let _cascade_view = shadow_map.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("cascade_{}", i)),
                    format: Some(wgpu::TextureFormat::Depth32Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: i,
                    array_layer_count: Some(1),
                    usage: None,
                });
            }

            // Create array view for sampling
            let _array_view = shadow_map.create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow_array"),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(CASCADE_COUNT),
                usage: None,
            });
        });
    }

    #[test]
    fn test_shadow_comparison_sampler() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("shadow_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            });

            drop(sampler);
        });
    }

    /// Cascade split uniform buffer
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct CascadeSplits {
        splits: [f32; 4],
    }

    #[test]
    fn test_cascade_splits_buffer() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let splits = CascadeSplits {
                splits: [10.0, 25.0, 50.0, 100.0],
            };

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cascade_splits"),
                contents: bytemuck::bytes_of(&splits),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            assert_eq!(buffer.size(), 16);
        });
    }
}

// =============================================================================
// Draw Call Validation Tests
// =============================================================================

mod draw_tests {
    use super::*;

    const SIMPLE_VS: &str = r#"
        @vertex
        fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
            return vec4<f32>(pos, 1.0);
        }
    "#;

    const SIMPLE_FS: &str = r#"
        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct SimpleVertex {
        position: [f32; 3],
    }

    #[test]
    fn test_draw_triangle() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            let vs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_VS.into()),
            });

            let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_FS.into()),
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    }],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

            let vertices = [
                SimpleVertex {
                    position: [0.0, 0.5, 0.0],
                },
                SimpleVertex {
                    position: [-0.5, -0.5, 0.0],
                },
                SimpleVertex {
                    position: [0.5, -0.5, 0.0],
                },
            ];

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            // Create render target
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("target"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });

            let view = target.create_view(&wgpu::TextureViewDescriptor::default());

            // Draw
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..3, 0..1);
            }

            queue.submit(Some(encoder.finish()));
            let _ = device.poll(wgpu::MaintainBase::Wait);
        });
    }

    #[test]
    fn test_indexed_draw() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            let vs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_VS.into()),
            });

            let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_FS.into()),
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    }],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

            // Quad vertices
            let vertices = [
                SimpleVertex {
                    position: [-0.5, 0.5, 0.0],
                },
                SimpleVertex {
                    position: [-0.5, -0.5, 0.0],
                },
                SimpleVertex {
                    position: [0.5, -0.5, 0.0],
                },
                SimpleVertex {
                    position: [0.5, 0.5, 0.0],
                },
            ];

            let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("target"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let view = target.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..6, 0, 0..1);
            }

            queue.submit(Some(encoder.finish()));
            let _ = device.poll(wgpu::MaintainBase::Wait);
        });
    }

    #[test]
    fn test_instanced_draw() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            const INSTANCED_VS: &str = r#"
                struct InstanceData {
                    @location(1) offset: vec2<f32>,
                }

                @vertex
                fn vs_main(
                    @location(0) pos: vec3<f32>,
                    instance: InstanceData,
                ) -> @builtin(position) vec4<f32> {
                    return vec4<f32>(pos.xy + instance.offset, pos.z, 1.0);
                }
            "#;

            let vs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vs"),
                source: wgpu::ShaderSource::Wgsl(INSTANCED_VS.into()),
            });

            let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_FS.into()),
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        // Vertex buffer
                        wgpu::VertexBufferLayout {
                            array_stride: 12,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            }],
                        },
                        // Instance buffer
                        wgpu::VertexBufferLayout {
                            array_stride: 8,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &[wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 1,
                            }],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

            let vertices = [
                SimpleVertex {
                    position: [0.0, 0.1, 0.0],
                },
                SimpleVertex {
                    position: [-0.1, -0.1, 0.0],
                },
                SimpleVertex {
                    position: [0.1, -0.1, 0.0],
                },
            ];

            // Instance offsets
            let instances: [[f32; 2]; 4] = [[-0.5, 0.5], [0.5, 0.5], [-0.5, -0.5], [0.5, -0.5]];

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("target"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let view = target.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, instance_buffer.slice(..));
                pass.draw(0..3, 0..4); // 3 vertices, 4 instances
            }

            queue.submit(Some(encoder.finish()));
            let _ = device.poll(wgpu::MaintainBase::Wait);
        });
    }
}

// =============================================================================
// Depth Testing Tests
// =============================================================================

mod depth_tests {
    use super::*;

    #[test]
    fn test_depth_buffer_creation() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width: 1920,
                    height: 1080,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let _view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("depth_view"),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                ..Default::default()
            });
        });
    }

    #[test]
    fn test_depth_stencil_buffer() {
        pollster::block_on(async {
            let Some((device, _queue)) = create_test_device().await else {
                return;
            };

            let depth_stencil = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_stencil"),
                size: wgpu::Extent3d {
                    width: 800,
                    height: 600,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            // Depth-only view
            let _depth_view = depth_stencil.create_view(&wgpu::TextureViewDescriptor {
                label: Some("depth_only"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                ..Default::default()
            });

            // Stencil-only view
            let _stencil_view = depth_stencil.create_view(&wgpu::TextureViewDescriptor {
                label: Some("stencil_only"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::StencilOnly,
                ..Default::default()
            });
        });
    }
}
