//! Backend-Specific Golden Image Tests
//!
//! Validates renderer output across different GPU backends (DX12, Vulkan, Metal)
//! to ensure consistent, deterministic rendering.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// GPU backend identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuBackend {
    Dx12,
    Vulkan,
    Metal,
    Gl,
    WebGpu,
    Unknown,
}

impl GpuBackend {
    /// Detect backend from adapter info
    pub fn detect(adapter: &wgpu::Adapter) -> Self {
        let info = adapter.get_info();
        match info.backend {
            wgpu::Backend::Dx12 => GpuBackend::Dx12,
            wgpu::Backend::Vulkan => GpuBackend::Vulkan,
            wgpu::Backend::Metal => GpuBackend::Metal,
            wgpu::Backend::Gl => GpuBackend::Gl,
            wgpu::Backend::BrowserWebGpu => GpuBackend::WebGpu,
            _ => GpuBackend::Unknown,
        }
    }

    /// Get expected golden hash file suffix
    pub fn suffix(&self) -> &'static str {
        match self {
            GpuBackend::Dx12 => "dx12",
            GpuBackend::Vulkan => "vulkan",
            GpuBackend::Metal => "metal",
            GpuBackend::Gl => "gl",
            GpuBackend::WebGpu => "webgpu",
            GpuBackend::Unknown => "unknown",
        }
    }
}

/// Compute deterministic hash of RGBA image data
pub fn hash_image(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Compare two images, return (max_diff, avg_diff)
pub fn compare_images(a: &[u8], b: &[u8]) -> (u8, f32) {
    assert_eq!(a.len(), b.len(), "Image sizes must match");
    if a.is_empty() {
        return (0, 0.0);
    }

    let mut sum = 0u64;
    let mut max_diff = 0u8;

    for i in 0..a.len() {
        let diff = a[i].abs_diff(b[i]);
        sum += diff as u64;
        max_diff = max_diff.max(diff);
    }

    (max_diff, sum as f32 / a.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create test device and queue for GPU tests (copied from renderer_tests.rs)
    async fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        gpu_capability::request_headless(
            "test_device",
            wgpu::Features::empty(),
            wgpu::Limits::downlevel_defaults(),
            wgpu::PowerPreference::default(),
            true,
        )
        .await
        .map(|gpu| (gpu.device, gpu.queue))
    }

    #[test]
    fn test_backend_detection() {
        pollster::block_on(async {
            let Some(gpu) = gpu_capability::request_headless(
                "backend-detect",
                wgpu::Features::empty(),
                wgpu::Limits::downlevel_defaults(),
                wgpu::PowerPreference::LowPower,
                true,
            )
            .await
            else {
                return;
            };

            let backend = GpuBackend::detect(&gpu.adapter);
            println!(
                "Detected backend: {:?} (suffix: {})",
                backend,
                backend.suffix()
            );
        });
    }

    #[test]
    fn golden_triangle_render() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            let width = 64u32;
            let height = 64u32;

            // Create render target
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("golden-target"),
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
            let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

            // Simple triangle shader
            let shader_src = r#"
                @vertex
                fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
                    var pos = array<vec2<f32>, 3>(
                        vec2<f32>(0.0, 0.5),
                        vec2<f32>(-0.5, -0.5),
                        vec2<f32>(0.5, -0.5)
                    );
                    return vec4<f32>(pos[vid], 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(1.0, 0.5, 0.25, 1.0);
                }
            "#;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("golden-shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("golden-pipeline"),
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
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

            // Render
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("golden-encoder"),
            });

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("golden-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.1,
                                b: 0.2,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rpass.set_pipeline(&pipeline);
                rpass.draw(0..3, 0..1);
            }

            // Read back
            let bytes_per_pixel = 4u64;
            let row_bytes = width as u64 * bytes_per_pixel;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;
            let padded_row = row_bytes.div_ceil(align) * align;

            let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("golden-readback"),
                size: padded_row * height as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded_row as u32),
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
            let _ = device.poll(wgpu::MaintainBase::Wait);

            // Map and read
            let slice = read_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
            let _ = device.poll(wgpu::MaintainBase::Wait);
            rx.recv().unwrap().unwrap();

            let data = slice.get_mapped_range();
            let mut image_data = vec![0u8; (width * height * 4) as usize];
            for y in 0..height as usize {
                let src_offset = y as u64 * padded_row;
                let dst_offset = y * width as usize * 4;
                image_data[dst_offset..dst_offset + row_bytes as usize]
                    .copy_from_slice(&data[src_offset as usize..(src_offset + row_bytes) as usize]);
            }
            drop(data);
            read_buffer.unmap();

            // Hash the result
            let image_hash = hash_image(&image_data);
            println!("Image hash = 0x{:016x}", image_hash);

            // Verify we got a valid render (not all black or white)
            let sum: u64 = image_data.iter().map(|&b| b as u64).sum();
            let avg = sum as f32 / image_data.len() as f32;
            assert!(avg > 20.0, "Image too dark, rendering may have failed");
            assert!(avg < 235.0, "Image too bright, rendering may have failed");

            println!("Golden triangle test passed!");
        });
    }

    #[test]
    fn golden_determinism_check() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            let width = 32u32;
            let height = 32u32;

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("determinism-target"),
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

            let render_and_hash = || {
                let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                }

                let bytes_per_pixel = 4u64;
                let row_bytes = width as u64 * bytes_per_pixel;
                let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;
                let padded_row = row_bytes.div_ceil(align) * align;

                let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: padded_row * height as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                encoder.copy_texture_to_buffer(
                    target.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &read_buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(padded_row as u32),
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
                let _ = device.poll(wgpu::MaintainBase::Wait);

                let slice = read_buffer.slice(..);
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                slice.map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                let _ = device.poll(wgpu::MaintainBase::Wait);
                rx.recv().unwrap().unwrap();

                let data = slice.get_mapped_range();
                let hash = hash_image(&data);
                drop(data);
                read_buffer.unmap();
                hash
            };

            // Render 5 times and verify all hashes match
            let hashes: Vec<u64> = (0..5).map(|_| render_and_hash()).collect();

            for (i, &h) in hashes.iter().enumerate() {
                println!("Frame {}: hash = 0x{:016x}", i, h);
            }

            let first = hashes[0];
            assert!(
                hashes.iter().all(|&h| h == first),
                "Non-deterministic rendering detected: {:?}",
                hashes
            );

            println!("Determinism check passed - all 5 frames identical");
        });
    }
}
