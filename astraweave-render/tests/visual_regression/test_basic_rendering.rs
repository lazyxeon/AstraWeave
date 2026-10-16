//! Basic Visual Regression Tests
//!
//! Tests fundamental rendering operations like clear color, depth buffer, and simple geometry.

use super::visual_regression::VisualTestContext;

#[test]
fn test_clear_color() {
    let Some(ctx) = pollster::block_on(VisualTestContext::new(64, 64)) else {
        return;
    };

    let pixels = ctx.render_to_buffer(|device, queue, view, _width, _height| {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Color Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Color Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.3,
                            b: 0.7,
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

        queue.submit(Some(encoder.finish()));
    });

    // Verify that all pixels are the clear color (127, 76, 178, 255) with small tolerance for format conversions
    let expected_r = (0.5 * 255.0) as u8; // 127
    let expected_g = (0.3 * 255.0) as u8; // 76
    let expected_b = (0.7 * 255.0) as u8; // 178
    let expected_a = 255u8;

    for chunk in pixels.chunks_exact(4) {
        let delta_r = (chunk[0] as i16 - expected_r as i16).abs();
        let delta_g = (chunk[1] as i16 - expected_g as i16).abs();
        let delta_b = (chunk[2] as i16 - expected_b as i16).abs();
        let delta_a = (chunk[3] as i16 - expected_a as i16).abs();

        assert!(
            delta_r <= 2,
            "Red channel mismatch: expected {}, got {}",
            expected_r,
            chunk[0]
        );
        assert!(
            delta_g <= 2,
            "Green channel mismatch: expected {}, got {}",
            expected_g,
            chunk[1]
        );
        assert!(
            delta_b <= 2,
            "Blue channel mismatch: expected {}, got {}",
            expected_b,
            chunk[2]
        );
        assert!(
            delta_a <= 2,
            "Alpha channel mismatch: expected {}, got {}",
            expected_a,
            chunk[3]
        );
    }

    // Optionally save/compare with golden image
    ctx.assert_image_matches(&pixels, "tests/visual_regression/golden/clear_color.png", 2);
}

#[test]
fn test_depth_buffer() {
    let Some(ctx) = pollster::block_on(VisualTestContext::new(64, 64)) else {
        return;
    };

    let pixels = ctx.render_to_buffer(|device, queue, view, width, height| {
        // Create depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Buffer Test Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Depth Test Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth Test Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
            // Depth buffer is cleared but we're not rendering anything
        }

        queue.submit(Some(encoder.finish()));
    });

    // Verify all pixels are black (since we only cleared, didn't render)
    for chunk in pixels.chunks_exact(4) {
        assert_eq!(chunk[0], 0, "Red should be 0");
        assert_eq!(chunk[1], 0, "Green should be 0");
        assert_eq!(chunk[2], 0, "Blue should be 0");
        assert_eq!(chunk[3], 0, "Alpha should be 0");
    }

    ctx.assert_image_matches(
        &pixels,
        "tests/visual_regression/golden/depth_buffer.png",
        0,
    );
}

#[test]
fn test_simple_triangle() {
    let Some(ctx) = pollster::block_on(VisualTestContext::new(128, 128)) else {
        return;
    };

    let pixels = ctx.render_to_buffer(|device, queue, view, _width, _height| {
        // Create a simple shader that renders a triangle
        let shader_source = r#"
            @vertex
            fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
                var positions = array<vec2<f32>, 3>(
                    vec2<f32>(0.0, 0.5),
                    vec2<f32>(-0.5, -0.5),
                    vec2<f32>(0.5, -0.5)
                );
                return vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red
            }
        "#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Pipeline"),
            layout: None,
            cache: None,
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Triangle Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Triangle Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&pipeline);
            render_pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    });

    // Verify that we have some red pixels (triangle) and some white pixels (background)
    let mut has_red = false;
    let mut has_white = false;

    for chunk in pixels.chunks_exact(4) {
        if chunk[0] > 200 && chunk[1] < 50 && chunk[2] < 50 {
            has_red = true; // Red pixel
        }
        if chunk[0] > 200 && chunk[1] > 200 && chunk[2] > 200 {
            has_white = true; // White pixel
        }
    }

    assert!(has_red, "Should have red pixels (triangle)");
    assert!(has_white, "Should have white pixels (background)");

    ctx.assert_image_matches(
        &pixels,
        "tests/visual_regression/golden/simple_triangle.png",
        5,
    );
}
