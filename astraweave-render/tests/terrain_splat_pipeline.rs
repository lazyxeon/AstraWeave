//! GPU integration tests for the terrain splat-array pipeline manager.
//!
//! Gated behind BOTH the `terrain-splat-arrays` feature (exposes the manager)
//! and the `gpu-tests` feature (ensures a wgpu device is available). Without
//! either, the file is empty.

#![cfg(all(feature = "terrain-splat-arrays", feature = "gpu-tests"))]

use astraweave_render::{
    LayerTextures, TerrainMaterialConfig, TerrainMaterialGpu, TerrainMaterialManager,
    TerrainSplatVertex, MAX_TERRAIN_LAYERS,
};
use wgpu::util::DeviceExt;

mod test_utils;

/// Build a minimal chequerboard RGBA8 buffer for `resolution × resolution`.
fn checkerboard(resolution: u32, rgb_a: [u8; 3], rgb_b: [u8; 3]) -> Vec<u8> {
    let mut v = Vec::with_capacity((resolution * resolution) as usize * 4);
    for y in 0..resolution {
        for x in 0..resolution {
            let on = ((x / 4) + (y / 4)) % 2 == 0;
            let rgb = if on { rgb_a } else { rgb_b };
            v.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    v
}

#[test]
fn terrain_manager_creation_allocates_gpu_resources() {
    pollster::block_on(async {
        let (device, _queue) = test_utils::create_headless_device().await;
        let config = TerrainMaterialConfig {
            albedo_resolution: 64,
            aux_resolution: 32,
            layer_count: MAX_TERRAIN_LAYERS,
        };
        let manager = TerrainMaterialManager::new(&device, config).expect("manager");
        assert_eq!(manager.chunk_splat_count(), 0);
        assert_eq!(manager.config().layer_count, MAX_TERRAIN_LAYERS);
    });
}

#[test]
fn terrain_manager_uploads_material_with_missing_channels() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let config = TerrainMaterialConfig {
            albedo_resolution: 32,
            aux_resolution: 16,
            layer_count: 2,
        };
        let mut manager = TerrainMaterialManager::new(&device, config).expect("manager");

        let albedo_0 = checkerboard(32, [200, 50, 50], [50, 200, 50]);
        let layers = vec![
            LayerTextures {
                albedo: Some(&albedo_0),
                // Normal/orm/height omitted → manager should use fallbacks
                normal: None,
                orm: None,
                height: None,
            },
            LayerTextures::default(), // fully default layer 1
        ];

        let mut gpu_material = TerrainMaterialGpu::default();
        gpu_material.active_layer_count = 2;

        manager
            .set_material(&queue, &gpu_material, &layers)
            .expect("set_material");
    });
}

#[test]
fn terrain_manager_rejects_wrong_sized_layer_data() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let config = TerrainMaterialConfig {
            albedo_resolution: 32,
            aux_resolution: 16,
            layer_count: 1,
        };
        let mut manager = TerrainMaterialManager::new(&device, config).expect("manager");

        // Too-small albedo payload should be rejected before any GPU call.
        let bogus = vec![0u8; 4];
        let layers = vec![LayerTextures {
            albedo: Some(&bogus),
            ..Default::default()
        }];
        let gpu_material = TerrainMaterialGpu::default();
        let err = manager
            .set_material(&queue, &gpu_material, &layers)
            .expect_err("must reject");
        let msg = format!("{err}");
        assert!(msg.contains("albedo size mismatch"), "got: {msg}");
    });
}

#[test]
fn terrain_manager_registers_and_removes_chunk_splats() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let config = TerrainMaterialConfig {
            albedo_resolution: 16,
            aux_resolution: 16,
            layer_count: 2,
        };
        let mut manager = TerrainMaterialManager::new(&device, config).expect("manager");

        // 8×8 RGBA8 splat maps.
        let map = vec![64u8; 8 * 8 * 4];
        manager
            .set_chunk_splat(&device, &queue, 1, &map, &map, (8, 8))
            .expect("chunk 1");
        manager
            .set_chunk_splat(&device, &queue, 2, &map, &map, (8, 8))
            .expect("chunk 2");
        assert_eq!(manager.chunk_splat_count(), 2);
        assert_eq!(manager.chunk_splat_dims(1), Some((8, 8)));

        assert!(manager.remove_chunk_splat(1));
        assert!(!manager.remove_chunk_splat(1)); // already gone
        assert_eq!(manager.chunk_splat_count(), 1);

        manager.clear_chunks();
        assert_eq!(manager.chunk_splat_count(), 0);
    });
}

#[test]
fn terrain_manager_full_pipeline_records_draw_without_validation_errors() {
    pollster::block_on(async {
        let (device, queue) = test_utils::create_headless_device().await;
        let config = TerrainMaterialConfig {
            albedo_resolution: 16,
            aux_resolution: 16,
            layer_count: 2,
        };

        // Push a validation scope so any shader/pipeline errors become test
        // failures rather than surfacing as panics at device drop time.
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let mut manager = TerrainMaterialManager::new(&device, config).expect("manager");

        // Upload minimal material + splats.
        let layers = vec![LayerTextures::default(); 2];
        let mut gpu_material = TerrainMaterialGpu::default();
        gpu_material.active_layer_count = 2;
        manager
            .set_material(&queue, &gpu_material, &layers)
            .expect("set_material");
        let map = vec![128u8; 4 * 4 * 4];
        manager
            .set_chunk_splat(&device, &queue, 42, &map, &map, (4, 4))
            .expect("chunk splat");

        // Update the camera so the UBO has finite values (prevents NaN warnings).
        manager.update_camera(
            &queue,
            glam::Mat4::IDENTITY,
            glam::Mat4::IDENTITY,
            glam::Vec3::ZERO,
            glam::Vec3::NEG_Z,
            glam::Vec3::X,
        );

        // Build the pipeline for a common Bgra8 color target + depth32.
        manager.ensure_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
        );

        // Create off-screen color attachments (3 targets) + depth.
        let color_desc = wgpu::TextureDescriptor {
            label: Some("terrain-test-color"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let color = device.create_texture(&color_desc);
        let normal = device.create_texture(&color_desc);
        let orm = device.create_texture(&color_desc);
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain-test-depth"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_v = color.create_view(&Default::default());
        let normal_v = normal.create_view(&Default::default());
        let orm_v = orm.create_view(&Default::default());
        let depth_v = depth.create_view(&Default::default());

        // Build a single-triangle vertex buffer in the expected layout.
        let verts = [
            TerrainSplatVertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
            },
            TerrainSplatVertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
            },
            TerrainSplatVertex {
                position: [0.0, 0.0, 1.0],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
        ];
        let indices: [u32; 3] = [0, 1, 2];
        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain-test-vb"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain-test-ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Record the draw.
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain-test-pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &color_v,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &normal_v,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &orm_v,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_v,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let drew = manager.draw_chunk(&mut rpass, 42, &vb, &ib, indices.len() as u32);
            assert!(drew, "draw_chunk should succeed");
        }
        queue.submit(std::iter::once(encoder.finish()));

        if let Some(err) = device.pop_error_scope().await {
            panic!("GPU validation error during draw: {err}");
        }
    });
}

/// Phase 1.E.1.c — verify the forward-lit splat pipeline builds without
/// wgpu validation errors on a headless device. Complements the naga-level
/// `test_pbr_terrain_forward_validates_with_prefix` in `shader_validation.rs`:
/// naga catches WGSL bugs; this catches bind-group-layout and pipeline-state
/// mismatches that only surface at pipeline creation time.
#[test]
fn terrain_manager_forward_pipeline_builds_without_validation_errors() {
    pollster::block_on(async {
        let (device, _queue) = test_utils::create_headless_device().await;
        let config = TerrainMaterialConfig {
            albedo_resolution: 16,
            aux_resolution: 16,
            layer_count: MAX_TERRAIN_LAYERS,
        };
        let mut manager = TerrainMaterialManager::new(&device, config).expect("manager");

        device.push_error_scope(wgpu::ErrorFilter::Validation);

        // Build for the engine's forward-pass targets: Rgba16Float HDR +
        // Depth32Float (same as `hdr_view` / `depth` in Renderer).
        let _pipeline = manager.ensure_forward_pipeline(
            &device,
            wgpu::TextureFormat::Rgba16Float,
            Some(wgpu::TextureFormat::Depth32Float),
        );

        // Calling with the same formats must be idempotent.
        let _pipeline2 = manager.ensure_forward_pipeline(
            &device,
            wgpu::TextureFormat::Rgba16Float,
            Some(wgpu::TextureFormat::Depth32Float),
        );

        if let Some(err) = device.pop_error_scope().await {
            panic!("GPU validation error during forward pipeline build: {err}");
        }
    });
}
