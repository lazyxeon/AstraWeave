//! Bloom Post-Processing Integration Tests
//! Tests the bloom pipeline with headless GPU execution

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[cfg(feature = "bloom")]
use astraweave_render::{BloomConfig, BloomPipeline};

#[test]
#[cfg(feature = "bloom")]
fn test_bloom_pipeline_creation() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no adapter");
        let (device, _queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("no device");

        let config = BloomConfig::default();
        let pipeline = BloomPipeline::new(&device, config);
        assert!(pipeline.is_ok(), "BloomPipeline creation should succeed");
    });
}

#[test]
#[cfg(feature = "bloom")]
#[allow(clippy::field_reassign_with_default)]
fn test_bloom_config_validation() {
    let config = BloomConfig::default();
    assert!(config.validate().is_ok(), "Default config should be valid");

    let mut invalid = BloomConfig::default();
    invalid.threshold = 11.0; // Above max
    assert!(
        invalid.validate().is_err(),
        "Invalid threshold should fail validation"
    );

    invalid.threshold = 1.0;
    invalid.intensity = 1.5; // Above max
    assert!(
        invalid.validate().is_err(),
        "Invalid intensity should fail validation"
    );

    invalid.intensity = 0.1;
    invalid.mip_count = 9; // Above max
    assert!(
        invalid.validate().is_err(),
        "Invalid mip_count should fail validation"
    );
}

#[test]
#[cfg(feature = "bloom")]
fn test_bloom_execute() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("no device");

        let config = BloomConfig {
            threshold: 1.0,
            intensity: 0.1,
            mip_count: 3, // Small mip count for test
        };
        let pipeline = BloomPipeline::new(&device, config).expect("pipeline creation failed");

        // Create a dummy HDR input texture (512x512)
        let width = 512;
        let height = 512;
        let hdr_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test-hdr-input"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Execute bloom
        let result = pipeline.execute(&device, &queue, &hdr_view, width, height);
        assert!(result.is_ok(), "Bloom execution should succeed");
    });
}

#[test]
#[cfg(feature = "bloom")]
fn test_bloom_mip_clamp() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("no device");

        let config = BloomConfig {
            threshold: 1.0,
            intensity: 0.05,
            mip_count: 8, // Max requested
        };
        let pipeline = BloomPipeline::new(&device, config).expect("pipeline creation failed");

        // Small texture (64x64) should clamp mip count
        let width = 64;
        let height = 64;
        let hdr_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test-hdr-small"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Should succeed even with requested 8 mips on 64x64 texture (will clamp to 7)
        let result = pipeline.execute(&device, &queue, &hdr_view, width, height);
        assert!(
            result.is_ok(),
            "Bloom should handle mip clamping gracefully"
        );
    });
}

// Placeholder test for non-bloom feature path
#[test]
#[cfg(not(feature = "bloom"))]
fn test_bloom_disabled() {
    // When bloom feature is disabled, this test ensures the crate still compiles
    eprintln!("Bloom feature disabled; skipping bloom integration assertions.");
}
