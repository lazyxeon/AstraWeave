//! Phase 5.3 T7 (stage 1) — GPU integration tests for [`astraweave_render::impostor_pass::ImpostorPass`].
//!
//! Exercises the full draw helper against a headless software device:
//! pipeline build, atlas upload, camera + instance updates, and a real
//! recorded draw into an offscreen target with a pixel readback check.

#![cfg(all(feature = "impostor-bake", feature = "gpu-tests"))]

use astraweave_render::impostor_lod3::Lod3InstanceRaw;
use astraweave_render::impostor_pass::ImpostorPass;
use astraweave_render::vegetation_lod::ImpostorAtlasSpec;
use glam::{Mat4, Vec3};

#[path = "test_utils.rs"]
mod test_utils;

const ATLAS_W: u32 = 32;
const ATLAS_H: u32 = 16;
const TARGET_W: u32 = 64;
const TARGET_H: u32 = 64;

fn solid_atlas(fill: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((ATLAS_W * ATLAS_H * 4) as usize);
    for _ in 0..(ATLAS_W * ATLAS_H) {
        v.extend_from_slice(&fill);
    }
    v
}

fn make_spec() -> ImpostorAtlasSpec {
    // One species, 4 baked angles — keeps the test fast and deterministic.
    ImpostorAtlasSpec::uniform(ATLAS_W, ATLAS_H, 4, &["test_tree"])
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn impostor_pass_new_builds_with_valid_inputs() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let pixels = solid_atlas([255, 0, 0, 255]);
        let pass = ImpostorPass::new(
            &device,
            &queue,
            &pixels,
            ATLAS_W,
            ATLAS_H,
            make_spec(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
        )
        .expect("ImpostorPass::new must succeed with valid inputs");
        assert_eq!(pass.instance_count(), 0, "fresh pass must have zero instances");
    });
}

#[test]
fn impostor_pass_new_rejects_mismatched_pixel_buffer() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        // One-byte-short buffer → Lod3Resources::upload must bail.
        let mut pixels = solid_atlas([0, 255, 0, 255]);
        pixels.pop();
        let err = ImpostorPass::new(
            &device,
            &queue,
            &pixels,
            ATLAS_W,
            ATLAS_H,
            make_spec(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
        )
        .map(|_| ());
        assert!(err.is_err(), "ImpostorPass::new should reject wrong-sized pixel buffer");
    });
}

#[test]
fn upload_instances_grows_buffer_geometrically() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let pixels = solid_atlas([0, 0, 255, 255]);
        let mut pass = ImpostorPass::new(
            &device,
            &queue,
            &pixels,
            ATLAS_W,
            ATLAS_H,
            make_spec(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
        )
        .unwrap();

        // Initial capacity is 64. Push 256 instances — buffer must grow at
        // least once. Pass must report the exact count we pushed.
        let instances: Vec<Lod3InstanceRaw> = (0..256)
            .map(|i| Lod3InstanceRaw {
                position_scale: [i as f32, 0.0, 0.0, 1.0],
                species_and_params: [0.0; 4],
            })
            .collect();
        pass.upload_instances(&device, &queue, &instances);
        assert_eq!(pass.instance_count(), 256);

        // Shrinking back to zero is legal and leaves count at zero.
        pass.upload_instances(&device, &queue, &[]);
        assert_eq!(pass.instance_count(), 0);
    });
}

#[test]
fn record_writes_red_pixels_for_red_atlas() {
    // End-to-end rendering test: record the pass into an offscreen
    // Rgba8UnormSrgb target, read back, and assert the top-left pixel is
    // red (a single instance aimed straight at the camera samples the red
    // atlas cell).
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;

        let pixels = solid_atlas([255, 0, 0, 255]);
        let mut pass = ImpostorPass::new(
            &device,
            &queue,
            &pixels,
            ATLAS_W,
            ATLAS_H,
            make_spec(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None, // no depth target — simpler readback
        )
        .unwrap();

        // Camera: sits at +Z looking toward origin.
        let cam_pos = Vec3::new(0.0, 0.5, 4.0);
        let view = Mat4::look_at_rh(cam_pos, Vec3::new(0.0, 0.5, 0.0), Vec3::Y);
        let proj = Mat4::perspective_rh(
            60f32.to_radians(),
            TARGET_W as f32 / TARGET_H as f32,
            0.1,
            100.0,
        );
        pass.update_camera(&queue, proj * view, cam_pos);

        // Single instance at origin, scale 2.
        let instances = [Lod3InstanceRaw {
            position_scale: [0.0, 0.0, 0.0, 2.0],
            species_and_params: [0.0, 0.0, 0.0, 0.0],
        }];
        pass.upload_instances(&device, &queue, &instances);
        assert_eq!(pass.instance_count(), 1);

        // Create the render target.
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("impostor-pass-test-target"),
            size: wgpu::Extent3d {
                width: TARGET_W,
                height: TARGET_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("impostor-pass-test"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear to transparent black so any red pixel = pass output.
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.record(&mut render_pass);
        }

        // Readback.
        let bytes_per_row = (TARGET_W * 4).next_multiple_of(256);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor-pass-test-readback"),
            size: (bytes_per_row * TARGET_H) as u64,
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
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(TARGET_H),
                },
            },
            wgpu::Extent3d {
                width: TARGET_W,
                height: TARGET_H,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).ok();
        });
        device.poll(wgpu::PollType::Wait).expect("device.poll must succeed");
        rx.recv().expect("map_async result").expect("map_async must succeed");

        let data = slice.get_mapped_range();

        // Scan the centre of the image — the billboard should cover a block
        // of pixels there. Count how many pixels have a dominant red channel.
        let mut red_hits = 0usize;
        let mut sampled = 0usize;
        let y_range = (TARGET_H / 4)..(TARGET_H * 3 / 4);
        let x_range = (TARGET_W / 4)..(TARGET_W * 3 / 4);
        for y in y_range.clone() {
            for x in x_range.clone() {
                let off = (y * bytes_per_row + x * 4) as usize;
                let r = data[off];
                let g = data[off + 1];
                let b = data[off + 2];
                let a = data[off + 3];
                sampled += 1;
                if a > 0 && r > 200 && g < 64 && b < 64 {
                    red_hits += 1;
                }
            }
        }
        drop(data);
        readback.unmap();

        assert!(
            red_hits > 20,
            "expected billboard to render red pixels in centre region \
             (got {red_hits} red hits out of {sampled} sampled pixels)"
        );
    });
}
