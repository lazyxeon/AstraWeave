//! The scene-vector pass stays off until a caller enables it.
//! An enabled frame stores the drawn world normal and zero motion on the
//! first frame. SSGI and TAA stay off.

use astraweave_render::Renderer;

fn f16_to_f32(h: u16) -> f32 {
    let sign = (h >> 15) & 1;
    let exp = (h >> 10) & 0x1f;
    let frac = h & 0x3ff;
    let bits = if exp == 0 {
        if frac == 0 {
            u32::from(sign) << 31
        } else {
            let mut e = 127 - 14;
            let mut f = frac;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            f &= 0x3ff;
            (u32::from(sign) << 31) | (e << 23) | (u32::from(f) << 13)
        }
    } else if exp == 31 {
        (u32::from(sign) << 31) | (0xff << 23) | (u32::from(frac) << 13)
    } else {
        (u32::from(sign) << 31) | ((u32::from(exp) + 127 - 15) << 23) | (u32::from(frac) << 13)
    };
    f32::from_bits(bits)
}

fn vertical_quad(renderer: &Renderer) -> astraweave_render::Mesh {
    let vertices = [
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, 0.5, 0.0],
    ];
    let normals = [[0.0, 0.0, 1.0]; 4];
    let indices = [0, 1, 2, 0, 2, 3];
    renderer.create_mesh_from_arrays(&vertices, &normals, &indices)
}

fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
) -> Vec<u8> {
    let row_bytes = width * bytes_per_pixel;
    let padded =
        row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene vector readback"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().expect("map callback").expect("map readback");
    let mapped = slice.get_mapped_range();
    let mut pixels = vec![0u8; (row_bytes * height) as usize];
    for y in 0..height as usize {
        let src = y * padded as usize;
        let dst = y * row_bytes as usize;
        pixels[dst..dst + row_bytes as usize]
            .copy_from_slice(&mapped[src..src + row_bytes as usize]);
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

#[test]
fn scene_vectors_stay_off_and_do_not_enable_ssgi_or_taa() {
    pollster::block_on(async {
        let renderer = Renderer::new_headless(64, 64).await.expect("headless");
        assert!(!renderer.scene_vectors_enabled());
        assert!(!renderer.ssgi_enabled());
        assert!(!renderer.taa_enabled());
        assert!(renderer.scene_vector_targets().is_none());
    });
}

#[test]
fn enabled_frame_stores_the_quad_normal_and_zero_motion() {
    pollster::block_on(async {
        const WIDTH: u32 = 64;
        let mut renderer = Renderer::new_headless(WIDTH, WIDTH)
            .await
            .expect("headless");
        assert!(!renderer.ssgi_enabled());
        assert!(!renderer.taa_enabled());
        let eye = glam::Vec3::new(0.0, 2.0, 4.0);
        let target = glam::Vec3::new(0.0, 2.0, 0.0);
        let projection =
            astraweave_camera::Projection::perspective(60.0_f32.to_radians(), 1.0, 0.1, 50.0);
        let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::Y);
        renderer.update_view(&astraweave_camera::RenderView::new(
            view,
            &projection,
            eye,
            (target - eye).normalize(),
        ));
        let mesh = vertical_quad(&renderer);
        renderer.add_model(
            "quad",
            mesh,
            &[astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::new(0.0, 2.0, 0.0),
                glam::Vec3::splat(3.0),
                [1.0, 1.0, 1.0, 1.0],
            )],
        );
        renderer.set_scene_vectors_enabled(true);
        let device = renderer.device().clone();
        let queue = renderer.queue().clone();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene vector color"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: WIDTH,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene vectors"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw_into");
        let (normal, velocity) = renderer
            .scene_vector_targets()
            .expect("targets after an enabled frame");
        let normal_tex = normal.clone();
        let velocity_tex = velocity.clone();
        let normals = read_texture(&device, &queue, encoder, &normal_tex, WIDTH, WIDTH, 8);
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene velocity"),
        });
        let velocities = read_texture(&device, &queue, encoder, &velocity_tex, WIDTH, WIDTH, 4);
        assert!(!renderer.ssgi_enabled());
        assert!(!renderer.taa_enabled());

        let mut drawn = 0u32;
        let mut clear = 0u32;
        for y in 0..WIDTH {
            for x in 0..WIDTH {
                let i = ((y * WIDTH + x) * 8) as usize;
                let nx = f16_to_f32(u16::from_le_bytes([normals[i], normals[i + 1]]));
                let ny = f16_to_f32(u16::from_le_bytes([normals[i + 2], normals[i + 3]]));
                let nz = f16_to_f32(u16::from_le_bytes([normals[i + 4], normals[i + 5]]));
                let nw = f16_to_f32(u16::from_le_bytes([normals[i + 6], normals[i + 7]]));
                let v = ((y * WIDTH + x) * 4) as usize;
                let vx = f16_to_f32(u16::from_le_bytes([velocities[v], velocities[v + 1]]));
                let vy = f16_to_f32(u16::from_le_bytes([velocities[v + 2], velocities[v + 3]]));
                if nw > 0.5 {
                    drawn += 1;
                    assert!(
                        nx.abs() < 0.15 && ny.abs() < 0.15 && nz > 0.9,
                        "{nx} {ny} {nz}"
                    );
                    assert!(vx.abs() < 0.02 && vy.abs() < 0.02, "{vx} {vy}");
                } else {
                    clear += 1;
                    assert!(nx.abs() < 0.02 && ny.abs() < 0.02 && nz.abs() < 0.02);
                }
            }
        }
        assert!(drawn > 16, "drawn pixels {drawn}");
        assert!(clear > 16, "cleared pixels {clear}");
    });
}
