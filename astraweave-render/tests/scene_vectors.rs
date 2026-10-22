//! The scene-vector pass stays off until a caller enables it.
//! An enabled frame stores a view-space normal packed into [0, 1] and a
//! UV motion delta. The first frame is zero motion. SSGI and TAA stay off.

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
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene vectors"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw_into");
        renderer.queue().submit(std::iter::once(encoder.finish()));
        let _ = device.poll(wgpu::MaintainBase::Wait);
        let validation = device.pop_error_scope().await;
        assert!(validation.is_none(), "validation {validation:?}");
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene vectors read"),
        });
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

        let mut covered = 0u32;
        let mut sample_n = glam::Vec3::ZERO;
        for y in 0..WIDTH {
            for x in 0..WIDTH {
                let sample = pixel_rg(&normals, &velocities, WIDTH, x, y);
                if sample.coverage > 0.5 {
                    covered += 1;
                    sample_n = sample.normal;
                }
            }
        }
        assert!(covered > 16, "covered {covered} sample {sample_n}");
        let center = pixel_rg(&normals, &velocities, WIDTH, WIDTH / 2, WIDTH / 2);
        let decoded = center.normal * 2.0 - glam::Vec3::ONE;
        assert!(
            decoded.x.abs() < 0.15 && decoded.y.abs() < 0.15 && decoded.z > 0.9,
            "view normal {decoded} covered {covered}"
        );
        assert!(
            center.velocity.x.abs() < 0.02 && center.velocity.y.abs() < 0.02,
            "first frame velocity {}",
            center.velocity
        );
        let mut clear = 0u32;
        for y in 0..WIDTH {
            for x in 0..WIDTH {
                let sample = pixel_rg(&normals, &velocities, WIDTH, x, y);
                if sample.coverage < 0.5 {
                    clear += 1;
                }
            }
        }
        assert!(clear > 16, "cleared pixels {clear}");
    });
}

struct Sample {
    normal: glam::Vec3,
    velocity: glam::Vec2,
    coverage: f32,
}

fn pixel_rg(normals: &[u8], velocities: &[u8], width: u32, x: u32, y: u32) -> Sample {
    let i = ((y * width + x) * 8) as usize;
    let v = ((y * width + x) * 4) as usize;
    Sample {
        normal: glam::Vec3::new(
            f16_to_f32(u16::from_le_bytes([normals[i], normals[i + 1]])),
            f16_to_f32(u16::from_le_bytes([normals[i + 2], normals[i + 3]])),
            f16_to_f32(u16::from_le_bytes([normals[i + 4], normals[i + 5]])),
        ),
        velocity: glam::Vec2::new(
            f16_to_f32(u16::from_le_bytes([velocities[v], velocities[v + 1]])),
            f16_to_f32(u16::from_le_bytes([velocities[v + 2], velocities[v + 3]])),
        ),
        coverage: f16_to_f32(u16::from_le_bytes([normals[i + 6], normals[i + 7]])),
    }
}

fn uv_of(vp: glam::Mat4, world: glam::Vec3) -> glam::Vec2 {
    let clip = vp * world.extend(1.0);
    let ndc = clip.truncate().truncate() / clip.w;
    glam::Vec2::new(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5)
}

#[test]
fn second_frame_matches_the_uv_delta_and_skips_a_culled_model() {
    pollster::block_on(async {
        const WIDTH: u32 = 64;
        let mut renderer = Renderer::new_headless(WIDTH, WIDTH)
            .await
            .expect("headless");
        let projection =
            astraweave_camera::Projection::perspective(60.0_f32.to_radians(), 1.0, 0.1, 50.0);
        let point = glam::Vec3::new(0.0, 2.0, 0.0);
        let aim = |renderer: &mut Renderer, eye: glam::Vec3| -> glam::Mat4 {
            let view = glam::Mat4::look_at_rh(eye, point, glam::Vec3::Y);
            renderer.update_view(&astraweave_camera::RenderView::new(
                view,
                &projection,
                eye,
                (point - eye).normalize(),
            ));
            renderer.current_view_proj()
        };
        let first_vp = aim(&mut renderer, glam::Vec3::new(0.0, 2.0, 4.0));
        let rotation = glam::Quat::from_rotation_y(0.4);
        let instance = astraweave_render::Instance {
            transform: glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::new(2.0, 0.5, 3.0),
                rotation,
                point,
            ),
            color: [1.0, 1.0, 1.0, 1.0],
            material_id: 0,
        };
        renderer.add_model("quad", vertical_quad(&renderer), &[instance.clone()]);
        let cover = vertical_quad(&renderer);
        renderer.add_model_with_bounds(
            "cover",
            cover,
            &[astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::new(0.0, 2.0, 2.0),
                glam::Vec3::splat(8.0),
                [1.0, 0.0, 0.0, 1.0],
            )],
            [40.0, 40.0, 40.0],
            [41.0, 41.0, 41.0],
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
        let color = target.create_view(&wgpu::TextureViewDescriptor::default());
        let draw = |renderer: &mut Renderer| {
            let mut encoder =
                renderer
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("scene vectors"),
                    });
            renderer
                .draw_into(&color, None, &mut encoder)
                .expect("draw_into");
            renderer.queue().submit(std::iter::once(encoder.finish()));
            let _ = renderer.device().poll(wgpu::MaintainBase::Wait);
        };
        draw(&mut renderer);
        let second_eye = glam::Vec3::new(0.6, 2.0, 4.0);
        let second_vp = aim(&mut renderer, second_eye);
        draw(&mut renderer);
        let (normal, velocity) = renderer.scene_vector_targets().expect("targets");
        let normal_tex = normal.clone();
        let velocity_tex = velocity.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("read normals"),
        });
        let normals = read_texture(&device, &queue, encoder, &normal_tex, WIDTH, WIDTH, 8);
        encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("read velocity"),
        });
        let velocities = read_texture(&device, &queue, encoder, &velocity_tex, WIDTH, WIDTH, 4);
        let curr_uv = uv_of(second_vp, point);
        let prev_uv = uv_of(first_vp, point);
        let expected = curr_uv - prev_uv;
        let px = (curr_uv.x * WIDTH as f32).clamp(0.0, (WIDTH - 1) as f32) as u32;
        let py = (curr_uv.y * WIDTH as f32).clamp(0.0, (WIDTH - 1) as f32) as u32;
        let sample = pixel_rg(&normals, &velocities, WIDTH, px, py);
        assert!(sample.coverage > 0.5, "pixel {px},{py} was clear");
        let delta = sample.velocity - expected;
        assert!(
            delta.length() < 0.05,
            "velocity {} expected {expected} at {px},{py}",
            sample.velocity
        );
        let world_n = {
            let raw = instance.raw();
            let columns = raw.normal_matrix.map(glam::Vec3::from_array);
            columns[2].normalize()
        };
        let view = glam::Mat4::look_at_rh(second_eye, point, glam::Vec3::Y);
        let view_n = (view.transform_vector3(world_n)).normalize();
        let decoded = sample.normal * 2.0 - glam::Vec3::ONE;
        assert!(
            (decoded - view_n).length() < 0.2,
            "decoded {decoded} view {view_n}"
        );
        let shifted = astraweave_render::Instance {
            transform: glam::Mat4::from_translation(glam::Vec3::new(0.35, 0.0, 0.0))
                * instance.transform,
            ..instance
        };
        assert!(renderer.update_model_instances("quad", &[shifted.clone()]));
        draw(&mut renderer);
        let (normal, velocity) = renderer.scene_vector_targets().expect("targets");
        let normal_tex = normal.clone();
        let velocity_tex = velocity.clone();
        encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("read moved normals"),
        });
        let normals = read_texture(&device, &queue, encoder, &normal_tex, WIDTH, WIDTH, 8);
        encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("read moved velocity"),
        });
        let velocities = read_texture(&device, &queue, encoder, &velocity_tex, WIDTH, WIDTH, 4);
        let curr_world = shifted.transform.transform_point3(glam::Vec3::ZERO);
        let prev_world = instance.transform.transform_point3(glam::Vec3::ZERO);
        let object_delta = uv_of(second_vp, curr_world) - uv_of(second_vp, prev_world);
        let curr_uv = uv_of(second_vp, curr_world);
        let px = (curr_uv.x * WIDTH as f32).clamp(0.0, (WIDTH - 1) as f32) as u32;
        let py = (curr_uv.y * WIDTH as f32).clamp(0.0, (WIDTH - 1) as f32) as u32;
        let moved_sample = pixel_rg(&normals, &velocities, WIDTH, px, py);
        let object_error = moved_sample.velocity - object_delta;
        assert!(
            moved_sample.coverage > 0.5 && object_error.length() < 0.08,
            "object velocity {} expected {object_delta} err {object_error}",
            moved_sample.velocity
        );
        let camera_only = uv_of(second_vp, prev_world) - uv_of(first_vp, prev_world);
        assert!(
            (moved_sample.velocity - camera_only).length() > 0.02,
            "object motion collapsed to the camera delta"
        );
    });
}
