//! Optional presentation passes submit under lavapipe with a real depth view.
//! They stay off in `draw_into` unless a setter enables them, so the allocation
//! gate does not count their passes.

use astraweave_render::Renderer;

fn color_target(device: &wgpu::Device) -> wgpu::TextureView {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gated color"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    target.create_view(&wgpu::TextureViewDescriptor::default())
}

fn submit(renderer: &mut Renderer, view: &wgpu::TextureView) {
    let device = renderer.device().clone();
    let queue = renderer.queue().clone();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gated"),
    });
    renderer
        .draw_into(view, None, &mut encoder)
        .expect("draw_into");
    queue.submit(std::iter::once(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
}

#[test]
fn atmosphere_follows_the_measured_default() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64).await.expect("headless");
        let quality = astraweave_render::PresentationQuality::default();
        assert_eq!(renderer.atmosphere_enabled(), quality.atmosphere());
        assert!(quality.atmosphere());
        assert!(!astraweave_render::PresentationQuality::Measured.atmosphere());
        renderer.set_atmosphere_enabled(false);
        assert!(!renderer.atmosphere_enabled());
    });
}

#[test]
fn gated_passes_submit_with_scene_depth() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64).await.expect("headless");
        renderer.set_atmosphere_enabled(true);
        renderer.set_cloud_raymarch_enabled(true);
        renderer.set_ssgi_enabled(true);
        renderer.set_taa_enabled(true);
        renderer.set_temporal_upscale_enabled(true);
        renderer.set_dof_enabled(true);
        let device = renderer.device().clone();
        let view = color_target(&device);
        submit(&mut renderer, &view);
        submit(&mut renderer, &view);
    });
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

fn sample(pixels: &[u8], width: u32, x: u32, y: u32) -> [i32; 3] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i] as i32, pixels[i + 1] as i32, pixels[i + 2] as i32]
}

fn channel_distance(a: [i32; 3], b: [i32; 3]) -> i32 {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()
}

fn aces(x: f32) -> f32 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

fn srgb_byte(linear: f32) -> i32 {
    let x = linear.clamp(0.0, 1.0);
    let encoded = if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as i32
}

/// Tonemap a clear color the way the post pass does, then encode sRGB.
fn tonemapped_clear(rgb: [f32; 3]) -> [i32; 3] {
    const EXPOSURE: f32 = 1.35;
    [
        srgb_byte(aces(rgb[0] * EXPOSURE)),
        srgb_byte(aces(rgb[1] * EXPOSURE)),
        srgb_byte(aces(rgb[2] * EXPOSURE)),
    ]
}

fn draw_and_read(renderer: &mut Renderer, atmosphere: bool) -> Vec<u8> {
    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    renderer.set_atmosphere_enabled(atmosphere);
    let device = renderer.device().clone();
    let queue = renderer.queue().clone();
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("atmosphere readback"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("atmosphere frame"),
    });
    renderer
        .draw_into(&view, None, &mut encoder)
        .expect("draw_into");
    let row_bytes = WIDTH * 4;
    let padded =
        row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("atmosphere pixels"),
        size: u64::from(padded) * u64::from(HEIGHT),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
    let err = pollster::block_on(device.pop_error_scope());
    assert!(
        err.is_none(),
        "validation rejected the atmosphere frame: {err:?}"
    );
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().expect("map callback").expect("map readback");
    let mapped = slice.get_mapped_range();
    let mut pixels = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT as usize {
        let src = y * padded as usize;
        let dst = y * row_bytes as usize;
        pixels[dst..dst + row_bytes as usize]
            .copy_from_slice(&mapped[src..src + row_bytes as usize]);
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

/// A cleared pixel is Bruneton sky. A near red quad stays red. A far red quad
/// stays geometry (depth is not the clear value) and aerial perspective
/// changes it relative to the same quad with the pass off.
#[test]
fn atmosphere_sky_differs_from_geometry_and_aerial_tints_distance() {
    pollster::block_on(async {
        const WIDTH: u32 = 64;
        let mut renderer = Renderer::new_headless(WIDTH, WIDTH)
            .await
            .expect("headless");
        renderer.set_debug_shading(1, false);
        renderer.set_material_params([1.0, 1.0, 1.0, 1.0], 0.0, 1.0);
        renderer.set_light_direction_override(glam::Vec3::NEG_Y, 1.0);
        let eye = glam::Vec3::new(0.0, 10.0, 5.0);
        let target = glam::Vec3::new(0.0, 10.0, -1.0);
        let projection =
            astraweave_camera::Projection::perspective(60.0_f32.to_radians(), 1.0, 0.1, 20_000.0);
        let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::Y);
        renderer.update_view(&astraweave_camera::RenderView::new(
            view,
            &projection,
            eye,
            (target - eye).normalize(),
        ));
        let mesh = vertical_quad(&renderer);
        renderer.add_model(
            "near",
            mesh.clone(),
            &[astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::new(-18.0, 10.0, -40.0),
                glam::Vec3::new(12.0, 12.0, 1.0),
                [1.0, 0.0, 0.0, 1.0],
            )],
        );
        renderer.add_model(
            "far",
            mesh,
            &[astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::new(0.0, 10.0, -8_000.0),
                glam::Vec3::new(4_000.0, 4_000.0, 1.0),
                [1.0, 0.0, 0.0, 1.0],
            )],
        );

        let off = draw_and_read(&mut renderer, false);
        let on = draw_and_read(&mut renderer, true);
        let near = sample(&on, WIDTH, 10, 32);
        let far = sample(&on, WIDTH, 32, 32);
        let sky = sample(&on, WIDTH, 60, 32);
        let sky_off = sample(&off, WIDTH, 60, 32);
        let far_off = sample(&off, WIDTH, 32, 32);
        let magenta = tonemapped_clear([1.0, 0.0, 1.0]);
        let grey = tonemapped_clear([0.05, 0.05, 0.05]);
        assert!(
            near[0] > near[1] + 40 && near[0] > near[2] + 40,
            "near geometry should stay red, got {near:?}"
        );
        assert!(
            channel_distance(sky, magenta) > 40,
            "sky {sky:?} is the debug magenta clear {magenta:?}"
        );
        assert!(
            channel_distance(sky, grey) > 40,
            "sky {sky:?} is the release grey clear {grey:?}"
        );
        assert!(
            channel_distance(sky, sky_off) > 40,
            "sky {sky:?} matched the HDRI sky {sky_off:?}"
        );
        assert!(
            channel_distance(sky, near) > 40,
            "sky {sky:?} replaced or matched the near quad {near:?}"
        );
        assert!(
            channel_distance(far, sky) > 40,
            "far geometry {far:?} was replaced by the sky {sky:?}"
        );
        assert!(
            channel_distance(far, far_off) > 20,
            "aerial perspective did not change the far quad: on {far:?} off {far_off:?} sky {sky:?}"
        );

        // Sun inside the shaft margin, on screen, upper right. A black quad
        // covers the sample and its y-mirror and stops short of the sun, so
        // the sample is not the tonemapped disk. Fog stays off. The march is
        // long enough to leave that quad into the sky from the upper pixel
        // and short enough to stay on the quad from the mirrored lower pixel.
        // A zero-weight shaft frame keeps aerial scattering off for both
        // reads, so the delta is the shaft add and not the next fog step.
        let toward_sun = glam::Vec3::new(0.25, 0.48, -1.0).normalize();
        renderer.set_light_direction_override(-toward_sun, 1.0);
        renderer.update_view(&astraweave_camera::RenderView::new(
            view,
            &projection,
            eye,
            (target - eye).normalize(),
        ));
        let sun_uv = astraweave_render::sun_to_screen(projection.matrix * view, toward_sun)
            .expect("sun inside the shaft margin");
        assert!(
            (0.55..0.95).contains(&sun_uv[0]) && (0.02..0.22).contains(&sun_uv[1]),
            "sun uv {sun_uv:?} should be on-screen upper right"
        );
        let sun_x = (sun_uv[0] * WIDTH as f32).floor() as u32;
        let sun_y = (sun_uv[1] * WIDTH as f32).floor() as u32;
        let upper_y = sun_y + 6;
        let lower_y = WIDTH - 1 - upper_y;
        let row_top = sun_y + 4;
        let row_bottom = (lower_y + 3).min(WIDTH - 1);
        assert!(
            upper_y + 8 < lower_y && row_top > sun_y,
            "rows do not leave a sky gap: sun {sun_y} upper {upper_y} lower {lower_y}"
        );
        let upper_uv = (upper_y as f32 + 0.5) / WIDTH as f32;
        let lower_uv = (lower_y as f32 + 0.5) / WIDTH as f32;
        let edge_uv = row_top as f32 / WIDTH as f32;
        let dist_upper = upper_uv - sun_uv[1];
        let density = (upper_uv - edge_uv + 2.0 / WIDTH as f32) / dist_upper;
        let lower_end = lower_uv - (lower_uv - sun_uv[1]) * density;
        assert!(
            (0.4..0.9).contains(&density) && lower_end > edge_uv + 4.0 / WIDTH as f32,
            "march window collapsed: density {density} lower end {lower_end} edge {edge_uv} sun {sun_uv:?}"
        );
        let distance = 24.0_f32;
        let tan_half = 30.0_f32.to_radians().tan();
        let world_at = |px: f32, py: f32| -> glam::Vec3 {
            let ndc_x = px / WIDTH as f32 * 2.0 - 1.0;
            let ndc_y = 1.0 - py / WIDTH as f32 * 2.0;
            glam::Vec3::new(
                eye.x + ndc_x * tan_half * distance,
                eye.y + ndc_y * tan_half * distance,
                eye.z - distance,
            )
        };
        let corner_lo = world_at(sun_x as f32 - 8.0, row_top as f32);
        let corner_hi = world_at(sun_x as f32 + 9.0, (row_bottom + 1) as f32);
        renderer.add_model(
            "shaft_quad",
            vertical_quad(&renderer),
            &[astraweave_render::Instance::from_pos_scale_color(
                (corner_lo + corner_hi) * 0.5,
                glam::Vec3::new(
                    (corner_hi.x - corner_lo.x).abs(),
                    (corner_hi.y - corner_lo.y).abs(),
                    1.0,
                ),
                [0.0, 0.0, 0.0, 1.0],
            )],
        );
        let upper = (sun_x, upper_y);
        let lower = (sun_x, lower_y);
        assert!(
            !renderer.volumetric_fog_enabled(),
            "shaft assert needs fog off"
        );
        renderer.set_god_ray_config(astraweave_render::GodRayConfig {
            enabled: true,
            half_res: false,
            num_samples: 4,
            density,
            weight: 0.0,
            decay: 0.9,
            exposure: 0.0,
        });
        assert!(renderer.god_rays_enabled(), "shaft pipeline did not build");
        let frame_only = draw_and_read(&mut renderer, true);
        let up_only = sample(&frame_only, WIDTH, upper.0, upper.1);
        let lo_only = sample(&frame_only, WIDTH, lower.0, lower.1);
        assert!(
            up_only.iter().sum::<i32>() < 48 && lo_only.iter().sum::<i32>() < 48,
            "sample missed the black quad. upper {up_only:?} at {upper:?} lower {lo_only:?} at {lower:?} sun {sun_uv:?}"
        );
        renderer.set_god_ray_config(astraweave_render::GodRayConfig {
            enabled: true,
            half_res: false,
            num_samples: 4,
            density,
            weight: 1.0,
            decay: 0.9,
            exposure: 4.0,
        });
        let frame_shaft = draw_and_read(&mut renderer, true);
        let up_shaft = sample(&frame_shaft, WIDTH, upper.0, upper.1);
        let lo_shaft = sample(&frame_shaft, WIDTH, lower.0, lower.1);
        let up_luma = up_shaft[0] + up_shaft[1] + up_shaft[2];
        let gain_upper = up_luma - (up_only[0] + up_only[1] + up_only[2]);
        let gain_lower =
            (lo_shaft[0] + lo_shaft[1] + lo_shaft[2]) - (lo_only[0] + lo_only[1] + lo_only[2]);
        assert!(
            up_luma > 30 && up_shaft != [0, 0, 0],
            "shaft output is black on the upper row {up_shaft:?} at {upper:?} sun {sun_uv:?}"
        );
        assert!(
            gain_upper > gain_lower + 8,
            "shafts should brighten the upper row more than the y-mirrored row. \
             upper gain {gain_upper} {up_only:?}->{up_shaft:?} \
             lower gain {gain_lower} {lo_only:?}->{lo_shaft:?} sun {sun_uv:?}"
        );

        // Height fog on the same rows. The upper ray looks up and the lower
        // ray looks down. A flipped ndc.y in fog_density.wgsl swaps them.
        renderer.set_god_ray_config(astraweave_render::GodRayConfig {
            enabled: true,
            half_res: false,
            num_samples: 4,
            density,
            weight: 0.0,
            decay: 0.9,
            exposure: 0.0,
        });
        renderer.set_volumetric_fog_config(
            astraweave_render::volumetric_fog::VolumetricFogConfig {
                enabled: true,
                froxel_dims: astraweave_render::volumetric_fog::VolumetricQuality::Low
                    .froxel_dims(),
                base_density: 0.0,
                height_fog_density: 0.8,
                height_fog_falloff: 1.2,
                height_fog_offset: eye.y,
                noise_intensity: 0.0,
                noise_speed: 0.0,
                anisotropy: -0.3,
                scatter_strength: 0.35,
                temporal_blend: 0.0,
                ..astraweave_render::volumetric_fog::VolumetricFogConfig::default()
            },
        );
        assert!(
            renderer.volumetric_fog_enabled(),
            "fog pipeline did not build"
        );
        let frame_fog = draw_and_read(&mut renderer, true);
        let up_fog = sample(&frame_fog, WIDTH, upper.0, upper.1);
        let lo_fog = sample(&frame_fog, WIDTH, lower.0, lower.1);
        let fog_upper =
            (up_fog[0] + up_fog[1] + up_fog[2]) - (up_only[0] + up_only[1] + up_only[2]);
        let fog_lower =
            (lo_fog[0] + lo_fog[1] + lo_fog[2]) - (lo_only[0] + lo_only[1] + lo_only[2]);
        assert!(
            fog_lower > fog_upper + 8,
            "height fog should sit on the lower row. A flipped ndc.y would swap them. \
             upper {up_only:?}->{up_fog:?} gain {fog_upper} \
             lower {lo_only:?}->{lo_fog:?} gain {fog_lower}"
        );
    });
}
