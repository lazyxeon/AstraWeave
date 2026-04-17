//! Phase 5.3 T1/T8 — GPU integration test for [`astraweave_render::impostor_bake`].
//!
//! Gated on `impostor-bake` + `gpu-tests` features; uses the shared headless
//! software adapter from `test_utils.rs`.

#![cfg(all(feature = "impostor-bake", feature = "gpu-tests"))]

use astraweave_render::impostor_bake::{
    fit_ortho_camera, upload_simplification_mesh, Aabb, ImpostorBaker, ImpostorBakerConfig,
};
use astraweave_render::lod_generator::SimplificationMesh;
use astraweave_render::vegetation_lod::AtlasRegion;
use glam::Vec3;
use wgpu::util::DeviceExt;

#[path = "test_utils.rs"]
mod test_utils;

/// Build a magenta 1×1 diffuse texture. Bright saturated colour makes the
/// rasterised pixels easy to detect in a readback without worrying about
/// sampling convention / sRGB conversion ambiguity at the atlas edges.
fn magenta_diffuse(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    let tex = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("magenta-diffuse"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::default(),
        &[255, 0, 255, 255],
    );
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Two-triangle quad centred at origin spanning [-0.5, 0.5] in X and Y,
/// with Z=0. Normals +Z, UVs span [0, 1].
fn unit_quad_mesh() -> SimplificationMesh {
    SimplificationMesh::new(
        vec![
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(-0.5, 0.5, 0.0),
        ],
        vec![Vec3::Z; 4],
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        vec![0, 1, 2, 0, 2, 3],
    )
}

#[test]
fn impostor_baker_creation_allocates_atlas() {
    let (device, _queue) = pollster::block_on(test_utils::create_headless_device());
    let baker = ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: 256,
            atlas_height: 256,
            sample_count: 1,
        },
    )
    .expect("baker creation");
    assert_eq!(baker.atlas_dimensions(), (256, 256));
    let size = baker.atlas().size();
    assert_eq!(size.width, 256);
    assert_eq!(size.height, 256);
}

#[test]
fn impostor_baker_rejects_zero_atlas_dimensions() {
    let (device, _queue) = pollster::block_on(test_utils::create_headless_device());
    assert!(ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: 0,
            atlas_height: 256,
            sample_count: 1,
        },
    )
    .is_err());
    assert!(ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: 256,
            atlas_height: 0,
            sample_count: 1,
        },
    )
    .is_err());
}

#[test]
fn impostor_baker_rejects_unsupported_msaa() {
    let (device, _queue) = pollster::block_on(test_utils::create_headless_device());
    let result = ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: 128,
            atlas_height: 128,
            sample_count: 4,
        },
    );
    assert!(result.is_err(), "MSAA > 1 not yet supported");
}

#[test]
fn impostor_baker_readback_is_zero_before_any_draw() {
    let (device, queue) = pollster::block_on(test_utils::create_headless_device());
    let baker = ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: 64,
            atlas_height: 64,
            sample_count: 1,
        },
    )
    .expect("baker");
    baker.clear(&device, &queue);
    let pixels = baker.readback_atlas(&device, &queue).expect("readback");
    assert_eq!(pixels.len(), 64 * 64 * 4);
    // After clear-to-transparent, every pixel should be fully transparent
    // black.
    let any_non_zero = pixels.iter().any(|&b| b != 0);
    assert!(!any_non_zero, "cleared atlas should be all zeros");
}

#[test]
fn impostor_baker_draws_quad_produces_non_zero_pixels_in_region() {
    let (device, queue) = pollster::block_on(test_utils::create_headless_device());
    let atlas_width = 128u32;
    let atlas_height = 128u32;
    let baker = ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width,
            atlas_height,
            sample_count: 1,
        },
    )
    .expect("baker");

    baker.clear(&device, &queue);

    let mesh = unit_quad_mesh();
    let (vbuf, ibuf, icount) = upload_simplification_mesh(&device, &mesh);
    let diffuse_view = magenta_diffuse(&device, &queue);
    let diffuse_bg = baker.make_diffuse_bind_group(&device, &diffuse_view);

    // Fit camera to the quad and bake into the top-left quarter of the atlas.
    let aabb = Aabb::from_points(&mesh.positions).expect("non-empty mesh");
    let (proj, view) = fit_ortho_camera(aabb, 0.0);
    let region = AtlasRegion {
        u_min: 0.0,
        v_min: 0.0,
        u_max: 0.5,
        v_max: 0.5,
    };

    baker.draw_into_region(
        &device,
        &queue,
        proj * view,
        region,
        &vbuf,
        &ibuf,
        icount,
        &diffuse_bg,
    );

    let pixels = baker.readback_atlas(&device, &queue).expect("readback");
    assert_eq!(pixels.len(), (atlas_width * atlas_height * 4) as usize);

    // Count non-transparent pixels inside the target region vs outside.
    let mut inside_opaque = 0u32;
    let mut outside_opaque = 0u32;
    let cell_w = (atlas_width / 2) as usize;
    let cell_h = (atlas_height / 2) as usize;
    for y in 0..atlas_height as usize {
        for x in 0..atlas_width as usize {
            let idx = (y * atlas_width as usize + x) * 4;
            let alpha = pixels[idx + 3];
            if alpha == 0 {
                continue;
            }
            if x < cell_w && y < cell_h {
                inside_opaque += 1;
            } else {
                outside_opaque += 1;
            }
        }
    }

    assert!(
        inside_opaque > 100,
        "expected the quad to fill a substantial portion of the target cell; got {} opaque pixels inside",
        inside_opaque
    );
    assert_eq!(
        outside_opaque, 0,
        "no opaque pixels should leak outside the target atlas region; got {}",
        outside_opaque
    );

    // Spot-check a pixel in the centre of the cell — expect near-magenta.
    let cx = cell_w / 2;
    let cy = cell_h / 2;
    let idx = (cy * atlas_width as usize + cx) * 4;
    let (r, g, b, a) = (pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3]);
    assert!(a > 250, "expected opaque centre pixel, got alpha={}", a);
    assert!(r > 200, "expected magenta red ~255, got {}", r);
    assert!(g < 40, "expected magenta green ~0, got {}", g);
    assert!(b > 200, "expected magenta blue ~255, got {}", b);
}

#[test]
fn impostor_baker_second_draw_preserves_first_cell() {
    // Ensures the load-vs-clear load-op choice keeps cells from previous
    // draws intact — the contract that makes multi-species / multi-angle
    // baking work in a single atlas.
    let (device, queue) = pollster::block_on(test_utils::create_headless_device());
    let baker = ImpostorBaker::new(
        &device,
        ImpostorBakerConfig {
            atlas_width: 128,
            atlas_height: 128,
            sample_count: 1,
        },
    )
    .expect("baker");
    baker.clear(&device, &queue);

    let mesh = unit_quad_mesh();
    let (vbuf, ibuf, icount) = upload_simplification_mesh(&device, &mesh);
    let diffuse_view = magenta_diffuse(&device, &queue);
    let diffuse_bg = baker.make_diffuse_bind_group(&device, &diffuse_view);
    let aabb = Aabb::from_points(&mesh.positions).unwrap();
    let (proj, view) = fit_ortho_camera(aabb, 0.0);

    // First: bake into top-left quarter.
    baker.draw_into_region(
        &device,
        &queue,
        proj * view,
        AtlasRegion {
            u_min: 0.0,
            v_min: 0.0,
            u_max: 0.5,
            v_max: 0.5,
        },
        &vbuf,
        &ibuf,
        icount,
        &diffuse_bg,
    );
    // Second: bake into bottom-right quarter.
    baker.draw_into_region(
        &device,
        &queue,
        proj * view,
        AtlasRegion {
            u_min: 0.5,
            v_min: 0.5,
            u_max: 1.0,
            v_max: 1.0,
        },
        &vbuf,
        &ibuf,
        icount,
        &diffuse_bg,
    );

    let pixels = baker.readback_atlas(&device, &queue).expect("readback");

    let sample = |x: usize, y: usize| -> u8 {
        let idx = (y * 128 + x) * 4;
        pixels[idx + 3]
    };
    // Top-left quarter should still have alpha (first bake not clobbered).
    assert!(sample(32, 32) > 0, "first cell was overwritten by second bake");
    // Bottom-right quarter should have alpha from the second bake.
    assert!(sample(96, 96) > 0, "second cell is empty");
}
