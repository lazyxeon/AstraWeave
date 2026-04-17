//! Phase 5.3 T4 — GPU integration test for [`astraweave_render::impostor_lod3`].
//!
//! Verifies that the LOD3 sampling pipeline + atlas resource helper can be
//! built end-to-end against a real wgpu device (software adapter in CI).

#![cfg(all(feature = "impostor-bake", feature = "gpu-tests"))]

use astraweave_render::impostor_lod3::{
    build_lod3_pipeline, Lod3Resources, Lod3SamplingConfig, SpeciesRowGpu,
};
use astraweave_render::vegetation_lod::ImpostorAtlasSpec;

#[path = "test_utils.rs"]
mod test_utils;

fn make_atlas_pixels(width: u32, height: u32, fill: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        v.extend_from_slice(&fill);
    }
    v
}

#[test]
fn lod3_pipeline_builds_on_headless_device() {
    pollster::block_on(async {
        let (device, _queue) = test_utils::create_headless_device().await;
        let pipeline = build_lod3_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
        )
        .expect("LOD3 pipeline must build");
        let _ = pipeline.pipeline; // touch field to prove it's populated
    });
}

#[test]
fn lod3_resources_upload_round_trips_spec_to_gpu() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let pipeline = build_lod3_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
        )
        .unwrap();

        let width = 64u32;
        let height = 32u32;
        let pixels = make_atlas_pixels(width, height, [0, 255, 0, 255]);
        let spec = ImpostorAtlasSpec::uniform(width, height, 4, &["oak", "pine"]);

        let res = Lod3Resources::upload(&device, &queue, &pixels, width, height, spec, &pipeline)
            .expect("upload must succeed with matching buffer size");

        // Config mirrors the spec we passed in.
        assert_eq!(res.config.rows.len(), 2);
        assert_eq!(res.config.rows[0], SpeciesRowGpu::from_spec(&res.config.spec, 0).unwrap());
        assert_eq!(res.config.spec.species[0].name, "oak");
        assert_eq!(res.config.spec.species[1].name, "pine");

        // Bind group + buffers are created (wgpu validates layouts eagerly).
        let _ = res.atlas_bind_group;
        let _ = res.rows_buffer;
    });
}

#[test]
fn lod3_resources_upload_rejects_wrong_sized_buffer() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let pipeline = build_lod3_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None,
        )
        .unwrap();

        let spec = ImpostorAtlasSpec::uniform(16, 16, 4, &["oak"]);
        let too_small = vec![0u8; 16 * 16 * 4 - 8];
        let result = Lod3Resources::upload(&device, &queue, &too_small, 16, 16, spec, &pipeline)
            .map(|_| ());
        let err = result.expect_err("upload must reject size mismatches");
        let msg = err.to_string();
        assert!(
            msg.contains("pixel buffer size mismatch"),
            "unexpected error: {msg}"
        );
    });
}

#[test]
fn lod3_resources_upload_rejects_zero_dimension_atlas() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let pipeline = build_lod3_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None,
        )
        .unwrap();

        let spec = ImpostorAtlasSpec::uniform(16, 16, 4, &["oak"]);
        let any = vec![0u8; 0];
        let result = Lod3Resources::upload(&device, &queue, &any, 0, 16, spec, &pipeline)
            .map(|_| ());
        let err = result.expect_err("zero-dimension atlas must fail");
        assert!(
            err.to_string().contains("non-zero")
                || err.to_string().contains("pixel buffer size mismatch"),
            "unexpected error: {err}"
        );
    });
}

/// Building a [`Lod3SamplingConfig`] from a spec then uploading and
/// re-deriving it must preserve the first-cell lookup semantics that the
/// shader depends on. Regression guard against subtle row-ordering bugs in
/// the upload path.
#[test]
fn lod3_resources_rows_match_cpu_lookup_after_upload() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let pipeline = build_lod3_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None,
        )
        .unwrap();

        let width = 128u32;
        let height = 64u32;
        let pixels = make_atlas_pixels(width, height, [128, 64, 200, 255]);
        let spec = ImpostorAtlasSpec::uniform(width, height, 8, &["a", "b", "c"]);
        let cfg_reference = Lod3SamplingConfig::from_spec(spec.clone());

        let res = Lod3Resources::upload(&device, &queue, &pixels, width, height, spec, &pipeline)
            .unwrap();

        assert_eq!(res.config.rows.len(), cfg_reference.rows.len());
        for (i, (a, b)) in res
            .config
            .rows
            .iter()
            .zip(cfg_reference.rows.iter())
            .enumerate()
        {
            assert_eq!(a, b, "row {i} mismatch between uploaded and reference config");
        }
    });
}
