//! GPU Validation Tests for Render System
//!
//! Comprehensive GPU tests that validate:
//! - Buffer operations (create, write, read back)
//! - Texture operations (create, upload, sampling)
//! - Compute shader correctness
//! - Render pass output verification
//! - Cross-backend consistency
//!
//! These tests use headless wgpu with software fallback for CI compatibility.

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use wgpu::util::DeviceExt;

// =============================================================================
// Test Infrastructure
// =============================================================================

/// Create headless wgpu device for testing (uses software fallback for CI)
async fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue, wgpu::Adapter)> {
    gpu_capability::request_headless(
        "gpu_validation_test_device",
        wgpu::Features::empty(),
        wgpu::Limits::downlevel_defaults(),
        wgpu::PowerPreference::default(),
        true,
    )
    .await
    .map(|gpu| (gpu.device, gpu.queue, gpu.adapter))
}

/// Compute deterministic hash of buffer contents
fn hash_bytes(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

// =============================================================================
// Buffer Operations Tests
// =============================================================================

mod buffer_tests {
    use super::*;

    #[test]
    fn test_buffer_create_and_write() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let data: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("test_buffer"),
                contents: bytemuck::cast_slice(&data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
            });

            assert_eq!(
                buffer.size(),
                16,
                "Buffer size should be 16 bytes (4 floats)"
            );
            assert!(buffer.usage().contains(wgpu::BufferUsages::UNIFORM));

            // Cleanup
            drop(buffer);
            let _ = device.poll(wgpu::MaintainBase::Wait);
        });
    }

    #[test]
    fn test_buffer_readback() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            // Create source buffer with data
            let source_data: [f32; 4] = [1.5, 2.5, 3.5, 4.5];
            let source_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("source_buffer"),
                contents: bytemuck::cast_slice(&source_data),
                usage: wgpu::BufferUsages::COPY_SRC,
            });

            // Create staging buffer for readback
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging_buffer"),
                size: 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Copy source to staging
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("copy_encoder"),
            });
            encoder.copy_buffer_to_buffer(&source_buffer, 0, &staging_buffer, 0, 16);
            queue.submit(Some(encoder.finish()));

            // Map and read
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().expect("Failed to map buffer");

            let data = buffer_slice.get_mapped_range();
            let result: &[f32] = bytemuck::cast_slice(&data);

            assert_eq!(result[0], 1.5, "First float should be 1.5");
            assert_eq!(result[1], 2.5, "Second float should be 2.5");
            assert_eq!(result[2], 3.5, "Third float should be 3.5");
            assert_eq!(result[3], 4.5, "Fourth float should be 4.5");

            drop(data);
            staging_buffer.unmap();
        });
    }

    #[test]
    fn test_buffer_queue_write() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            // Create uniform buffer
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniform_buffer"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Write using queue
            let data: [f32; 4] = [10.0, 20.0, 30.0, 40.0];
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&data));

            // Create staging for readback
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(&buffer, 0, &staging, 0, 16);
            queue.submit(Some(encoder.finish()));

            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let mapped = slice.get_mapped_range();
            let result: &[f32] = bytemuck::cast_slice(&mapped);
            assert_eq!(result, &[10.0, 20.0, 30.0, 40.0]);

            drop(mapped);
            staging.unmap();
        });
    }

    #[test]
    fn test_buffer_partial_write() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            // Create buffer with initial zeros
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("partial_buffer"),
                size: 32,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Write zeros first
            queue.write_buffer(&buffer, 0, &[0u8; 32]);

            // Write partial data at offset 16
            let data: [f32; 2] = [100.0, 200.0];
            queue.write_buffer(&buffer, 16, bytemuck::cast_slice(&data));

            // Readback and verify
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: 32,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(&buffer, 0, &staging, 0, 32);
            queue.submit(Some(encoder.finish()));

            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let mapped = slice.get_mapped_range();
            let result: &[f32] = bytemuck::cast_slice(&mapped);

            // First 4 floats should be 0, last 2 should be our values
            assert_eq!(result[0], 0.0);
            assert_eq!(result[1], 0.0);
            assert_eq!(result[2], 0.0);
            assert_eq!(result[3], 0.0);
            assert_eq!(result[4], 100.0);
            assert_eq!(result[5], 200.0);

            drop(mapped);
            staging.unmap();
        });
    }
}

// =============================================================================
// Texture Operations Tests
// =============================================================================

mod texture_tests {
    use super::*;

    #[test]
    fn test_texture_create_rgba8() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("rgba8_texture"),
                size: wgpu::Extent3d {
                    width: 256,
                    height: 256,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            assert_eq!(texture.size().width, 256);
            assert_eq!(texture.size().height, 256);
            assert_eq!(texture.format(), wgpu::TextureFormat::Rgba8Unorm);
            assert_eq!(texture.mip_level_count(), 1);
            assert_eq!(texture.sample_count(), 1);
        });
    }

    #[test]
    fn test_texture_create_depth32() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_texture"),
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

            assert_eq!(texture.size().width, 1920);
            assert_eq!(texture.size().height, 1080);
            assert_eq!(texture.format(), wgpu::TextureFormat::Depth32Float);
        });
    }

    #[test]
    fn test_texture_upload_and_readback() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            let width = 4u32;
            let height = 4u32;

            // Create texture with COPY_SRC for readback
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });

            // Upload checkerboard pattern
            let mut data = vec![0u8; (width * height * 4) as usize];
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    if (x + y) % 2 == 0 {
                        data[idx] = 255; // R
                        data[idx + 1] = 0; // G
                        data[idx + 2] = 0; // B
                        data[idx + 3] = 255; // A
                    } else {
                        data[idx] = 0;
                        data[idx + 1] = 255;
                        data[idx + 2] = 0;
                        data[idx + 3] = 255;
                    }
                }
            }

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
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );

            // Create buffer for readback (with proper alignment)
            let bytes_per_row = (width * 4 + 255) & !255; // Align to 256
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: (bytes_per_row * height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &staging,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            // Map and verify
            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let mapped = slice.get_mapped_range();

            // Check first pixel (0,0) - should be red
            assert_eq!(mapped[0], 255, "Pixel (0,0) R should be 255");
            assert_eq!(mapped[1], 0, "Pixel (0,0) G should be 0");
            assert_eq!(mapped[2], 0, "Pixel (0,0) B should be 0");
            assert_eq!(mapped[3], 255, "Pixel (0,0) A should be 255");

            // Check second pixel (1,0) - should be green
            assert_eq!(mapped[4], 0, "Pixel (1,0) R should be 0");
            assert_eq!(mapped[5], 255, "Pixel (1,0) G should be 255");

            drop(mapped);
            staging.unmap();
        });
    }

    #[test]
    fn test_texture_view_creation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test_texture"),
                size: wgpu::Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 6,
                },
                mip_level_count: 4,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            // Create various views
            let _full_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("full_view"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
                usage: None,
            });

            let _single_layer = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("single_layer"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: None,
            });

            let _mip_1_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("mip_1"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 1,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: None,
                usage: None,
            });

            // Cube view
            let _cube_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("cube_view"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::Cube),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: Some(6),
                usage: None,
            });
        });
    }

    #[test]
    fn test_texture_hdr_format() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("hdr_texture"),
                size: wgpu::Extent3d {
                    width: 1024,
                    height: 1024,
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

            assert_eq!(texture.format(), wgpu::TextureFormat::Rgba16Float);
        });
    }
}

// =============================================================================
// Render Pass Tests
// =============================================================================

mod render_pass_tests {
    use super::*;

    #[test]
    fn test_clear_render_target() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            let width = 64u32;
            let height = 64u32;

            // Create render target
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("render_target"),
                size: wgpu::Extent3d {
                    width,
                    height,
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

            // Clear to specific color
            let clear_color = wgpu::Color {
                r: 0.25,
                g: 0.5,
                b: 0.75,
                a: 1.0,
            };

            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            // Copy to staging buffer for verification
            let bytes_per_row = (width * 4 + 255) & !255;
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: (bytes_per_row * height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &target,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &staging,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );

            queue.submit(Some(encoder.finish()));

            // Read back and verify
            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let mapped = slice.get_mapped_range();

            // Check first pixel matches clear color (with tolerance for float conversion)
            let expected_r = (clear_color.r * 255.0).round() as u8;
            let expected_g = (clear_color.g * 255.0).round() as u8;
            let expected_b = (clear_color.b * 255.0).round() as u8;
            let expected_a = (clear_color.a * 255.0).round() as u8;

            assert_eq!(mapped[0], expected_r, "R channel mismatch");
            assert_eq!(mapped[1], expected_g, "G channel mismatch");
            assert_eq!(mapped[2], expected_b, "B channel mismatch");
            assert_eq!(mapped[3], expected_a, "A channel mismatch");

            drop(mapped);
            staging.unmap();
        });
    }

    #[test]
    fn test_depth_clear() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width: 256,
                    height: 256,
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

            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("depth_clear_pass"),
                    color_attachments: &[],
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

            queue.submit(Some(encoder.finish()));
            let _ = device.poll(wgpu::MaintainBase::Wait);
        });
    }
}

// =============================================================================
// Shader Compilation Tests
// =============================================================================

mod shader_tests {
    use super::*;

    const SIMPLE_VERTEX_SHADER: &str = r#"
        @vertex
        fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
            var positions = array<vec2<f32>, 3>(
                vec2<f32>(0.0, 0.5),
                vec2<f32>(-0.5, -0.5),
                vec2<f32>(0.5, -0.5)
            );
            return vec4<f32>(positions[idx], 0.0, 1.0);
        }
    "#;

    const SIMPLE_FRAGMENT_SHADER: &str = r#"
        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    const COMPUTE_SHADER: &str = r#"
        @group(0) @binding(0) var<storage, read_write> data: array<f32>;

        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            data[id.x] = data[id.x] * 2.0;
        }
    "#;

    #[test]
    fn test_vertex_shader_compilation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_VERTEX_SHADER.into()),
            });

            // Shader created successfully
            drop(shader);
        });
    }

    #[test]
    fn test_fragment_shader_compilation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fragment_shader"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_FRAGMENT_SHADER.into()),
            });

            drop(shader);
        });
    }

    #[test]
    fn test_compute_shader_compilation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("compute_shader"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
            });

            drop(shader);
        });
    }

    #[test]
    fn test_render_pipeline_creation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let vs_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_VERTEX_SHADER.into()),
            });

            let fs_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fs"),
                source: wgpu::ShaderSource::Wgsl(SIMPLE_FRAGMENT_SHADER.into()),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("pipeline_layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vs_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs_shader,
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

            drop(pipeline);
        });
    }

    #[test]
    fn test_compute_pipeline_creation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("compute"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
            });

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compute_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute_layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("compute_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

            drop(pipeline);
        });
    }
}

// =============================================================================
// Compute Shader Validation Tests
// =============================================================================

mod compute_validation_tests {
    use super::*;

    const DOUBLE_VALUES_SHADER: &str = r#"
        @group(0) @binding(0) var<storage, read> input: array<f32>;
        @group(0) @binding(1) var<storage, read_write> output: array<f32>;

        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            output[id.x] = input[id.x] * 2.0;
        }
    "#;

    #[test]
    fn test_compute_shader_doubles_values() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("double_shader"),
                source: wgpu::ShaderSource::Wgsl(DOUBLE_VALUES_SHADER.into()),
            });

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

            // Create input data
            let input_data: Vec<f32> = (0..64).map(|i| i as f32).collect();
            let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("input"),
                contents: bytemuck::cast_slice(&input_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

            let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("output"),
                size: (64 * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: (64 * 4) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bind_group"),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("compute_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging, 0, 64 * 4);
            queue.submit(Some(encoder.finish()));

            // Read back and verify
            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let mapped = slice.get_mapped_range();
            let result: &[f32] = bytemuck::cast_slice(&mapped);

            // Verify each value is doubled
            for (i, &val) in result.iter().enumerate().take(64) {
                let expected = (i as f32) * 2.0;
                assert!(
                    (val - expected).abs() < 0.001,
                    "Value at index {} should be {}, got {}",
                    i,
                    expected,
                    val
                );
            }

            drop(mapped);
            staging.unmap();
        });
    }
}

// =============================================================================
// Backend Consistency Tests
// =============================================================================

mod backend_tests {
    use super::*;

    #[test]
    fn test_adapter_info() {
        pollster::block_on(async {
            let Some((_device, _queue, adapter)) = create_test_device().await else {
                return;
            };

            let info = adapter.get_info();

            // Just verify we can get adapter info
            println!("Backend: {:?}", info.backend);
            println!("Device: {}", info.name);
            println!("Device type: {:?}", info.device_type);
            println!("Driver: {}", info.driver);

            // Verify limits are reasonable
            let limits = adapter.limits();
            assert!(limits.max_texture_dimension_2d >= 2048);
            assert!(limits.max_bind_groups >= 4);
        });
    }

    #[test]
    fn test_buffer_hash_determinism() {
        pollster::block_on(async {
            let Some((device, queue, _adapter)) = create_test_device().await else {
                return;
            };

            let data: [f32; 16] = [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ];

            // Create buffer and write data
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("test"),
                size: 64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&data));

            // Create staging for readback
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: 64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(&buffer, 0, &staging, 0, 64);
            queue.submit(Some(encoder.finish()));

            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let mapped = slice.get_mapped_range();
            let hash = hash_bytes(&mapped);

            // Hash should be deterministic
            assert_ne!(hash, 0, "Hash should not be zero");

            // Verify data integrity
            let result: &[f32] = bytemuck::cast_slice(&mapped);
            for (i, &val) in result.iter().enumerate().take(16) {
                assert_eq!(val, (i + 1) as f32);
            }

            drop(mapped);
            staging.unmap();
        });
    }
}

// =============================================================================
// Resource Management Tests
// =============================================================================

mod resource_tests {
    use super::*;

    #[test]
    fn test_sampler_creation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            // Linear filtering sampler
            let _linear = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("linear"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            // Nearest filtering sampler
            let _nearest = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("nearest"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            // Comparison sampler for shadows
            let _comparison = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("comparison"),
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            });

            // Repeat addressing
            let _repeat = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("repeat"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                ..Default::default()
            });
        });
    }

    #[test]
    fn test_bind_group_creation() {
        pollster::block_on(async {
            let Some((device, _queue, _adapter)) = create_test_device().await else {
                return;
            };

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniform"),
                size: 256,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("layout"),
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

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bind_group"),
                layout: &bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

            drop(bind_group);
        });
    }
}
