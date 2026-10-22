//! The Hi-Z copy stores depth into R32Float. A uniform clear stays uniform,
//! which is what SSR and SSGI sample. A second build matches the first.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::hiz_pyramid::HizPyramid;

fn read_r32(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<f32> {
    let padded = 256u32;
    let rows = height;
    let size = (padded * rows) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hiz readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("hiz copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(rows),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("map");
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().expect("recv").expect("mapped");
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        let row = (y * padded) as usize;
        for x in 0..width {
            let i = row + (x as usize) * 4;
            out.push(f32::from_le_bytes(data[i..i + 4].try_into().unwrap()));
        }
    }
    out
}

#[test]
fn depth_copy_stays_uniform_for_ssr_and_ssgi() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        if !astraweave_render::gpu_fault::depth_texture_load_supported(&device) {
            eprintln!(
                "skip depth_copy_stays_uniform_for_ssr_and_ssgi: this adapter does not support textureLoad on depth textures"
            );
            return;
        }
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let pyramid = HizPyramid::new(&device, 8, 8, &depth_view);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear depth"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.25),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        pyramid.build(&mut encoder);
        queue.submit(std::iter::once(encoder.finish()));
        let first = read_r32(&device, &queue, pyramid.texture(), 8, 8);
        assert!(
            first.iter().all(|v| (*v - 0.25).abs() < 1e-4),
            "mip 0 must stay the cleared depth, got {first:?}"
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rebuild"),
        });
        pyramid.build(&mut encoder);
        queue.submit(std::iter::once(encoder.finish()));
        let second = read_r32(&device, &queue, pyramid.texture(), 8, 8);
        assert_eq!(
            first, second,
            "a second build must not change the pyramid SSR samples"
        );
    });
}
