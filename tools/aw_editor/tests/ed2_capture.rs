//! ED-2 — viewport capture: determinism and round-trip proof.
//!
//! Concern 3 item 4 asks whether two captures taken from the same restored
//! station are byte-identical, and if not, to measure the delta and provide a
//! settled-capture route. This exercises the **real** capture path:
//! `ViewportRenderer::create_render_texture` -> `render` -> `capture_frame_png`,
//! which is the same sequence `ViewportWidget` performs each frame — the widget
//! only adds the egui presentation on top.
//!
//! ```text
//! cargo test -p aw_editor --profile release-fast --test ed2_capture -- --ignored --nocapture
//! ```

use anyhow::{Context, Result};
use astraweave_core::World;
use aw_editor_lib::viewport::{OrbitCamera, ViewportRenderer};
use glam::Vec3;
use std::sync::Arc;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

async fn acquire() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .context("no suitable wgpu adapter for the ED-2 capture test")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("ed2-capture-test"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 8,
                ..wgpu::Limits::default()
            },
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .context("request_device failed")?;
    Ok((Arc::new(device), Arc::new(queue)))
}

/// A station worth restoring — deliberately not the default view, so a failure
/// to restore shows up as a completely different image rather than a subtle one.
fn a_station() -> OrbitCamera {
    let mut c = OrbitCamera::default();
    c.set_focal_point(Vec3::new(4.0, 1.5, -7.0));
    c.set_distance(19.0);
    c.set_yaw(1.05);
    c.set_pitch(0.62);
    c.set_fov(55.0);
    c.set_aspect(WIDTH as f32, HEIGHT as f32);
    c
}

#[test]
#[ignore = "requires a GPU; run explicitly (see module docs)"]
fn ed2_two_captures_from_one_restored_station_are_identical() -> Result<()> {
    pollster::block_on(async {
        let (device, queue) = acquire().await?;
        let mut viewport = ViewportRenderer::new(device.clone(), queue.clone())
            .context("ViewportRenderer::new failed")?;
        viewport
            .init_engine_adapter()
            .await
            .context("init_engine_adapter failed")?;

        let texture = viewport
            .create_render_texture(WIDTH, HEIGHT)
            .context("create_render_texture failed")?;
        let world = World::new();

        let out = std::env::temp_dir().join("aw_ed2_capture");
        std::fs::create_dir_all(&out)?;
        let a = out.join("station_a.png");
        let b = out.join("station_b.png");

        // Leg A: restore the station from its captured state, render, capture.
        let pinned = a_station().capture_state();
        let mut cam = OrbitCamera::default();
        cam.set_aspect(WIDTH as f32, HEIGHT as f32);
        cam.apply_state(&pinned);
        for _ in 0..3 {
            viewport
                .render(&texture, &cam, &world, None, None, None, false, false, 0)
                .context("render A failed")?;
        }
        viewport
            .capture_frame_png(&texture, &a)
            .context("capture A")?;

        // Fly the camera somewhere else entirely, then restore the SAME state.
        cam.set_focal_point(Vec3::new(-400.0, 90.0, 250.0));
        cam.set_distance(900.0);
        cam.set_yaw(-2.7);
        cam.set_pitch(-0.3);
        for _ in 0..3 {
            viewport
                .render(&texture, &cam, &world, None, None, None, false, false, 0)
                .context("render (flown away) failed")?;
        }
        cam.apply_state(&pinned);
        // Run the smoother the way the widget does: if the restore drifted,
        // leg B would differ. This is Concern 1 asserted through pixels.
        for _ in 0..30 {
            cam.smooth_update(1.0 / 60.0);
        }
        for _ in 0..3 {
            viewport
                .render(&texture, &cam, &world, None, None, None, false, false, 0)
                .context("render B failed")?;
        }
        viewport
            .capture_frame_png(&texture, &b)
            .context("capture B")?;

        let bytes_a = std::fs::read(&a)?;
        let bytes_b = std::fs::read(&b)?;
        println!(
            "[ed2] A: {} ({} bytes)\n[ed2] B: {} ({} bytes)",
            a.display(),
            bytes_a.len(),
            b.display(),
            bytes_b.len()
        );

        // Measure rather than merely assert, so a non-identical result is
        // reported as a magnitude instead of a bare failure.
        let img_a = astraweave_asset::open_limited_image(&a)?.to_rgba8();
        let img_b = astraweave_asset::open_limited_image(&b)?.to_rgba8();
        assert_eq!(img_a.dimensions(), img_b.dimensions());
        let (mut diff_px, mut max_ch) = (0usize, 0u8);
        for (pa, pb) in img_a.pixels().zip(img_b.pixels()) {
            let d = (0..4).map(|i| pa.0[i].abs_diff(pb.0[i])).max().unwrap_or(0);
            if d > 0 {
                diff_px += 1;
                max_ch = max_ch.max(d);
            }
        }
        let total = (img_a.width() * img_a.height()) as usize;
        println!(
            "[ed2] differing pixels: {diff_px}/{total} ({:.4}%), max channel delta {max_ch}",
            100.0 * diff_px as f64 / total as f64
        );

        assert_eq!(
            diff_px, 0,
            "two captures from the same restored station must be identical \
             (differing pixels: {diff_px}, max channel delta {max_ch})"
        );
        assert_eq!(bytes_a, bytes_b, "encoded PNGs must be byte-identical");
        Ok(())
    })
}

#[test]
#[ignore = "requires a GPU; run explicitly (see module docs)"]
fn ed2_capture_writes_a_readable_png_of_the_requested_size() -> Result<()> {
    pollster::block_on(async {
        let (device, queue) = acquire().await?;
        let mut viewport = ViewportRenderer::new(device.clone(), queue.clone())?;
        viewport.init_engine_adapter().await?;
        let texture = viewport.create_render_texture(WIDTH, HEIGHT)?;
        let world = World::new();
        let cam = a_station();
        for _ in 0..2 {
            viewport.render(&texture, &cam, &world, None, None, None, false, false, 0)?;
        }
        let out = std::env::temp_dir().join("aw_ed2_capture").join("size.png");
        let (w, h) = viewport.capture_frame_png(&texture, &out)?;
        assert_eq!((w, h), (WIDTH, HEIGHT));
        let img = astraweave_asset::open_limited_image(&out)?;
        assert_eq!((img.width(), img.height()), (WIDTH, HEIGHT));
        println!("[ed2] wrote {} at {w}x{h}", out.display());
        Ok(())
    })
}
