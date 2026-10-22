//! IBL Integration Tests
//! Tests the Image-Based Lighting pipeline with CPU-safe defaults

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[cfg(feature = "ibl")]
use astraweave_render::{IblManager, IblQuality};

#[test]
#[cfg(feature = "ibl")]
fn test_ibl_manager_creation() {
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

        let manager = IblManager::new(&device, IblQuality::Low);
        assert!(manager.is_ok(), "IblManager creation should succeed");
        let mgr = manager.unwrap();

        // Verify bind group layout exists
        let _bgl = mgr.bind_group_layout();

        // Verify sampler exists
        let _sampler = mgr.sampler();
    });
}

#[test]
#[cfg(feature = "ibl")]
fn test_ibl_bake_environment() {
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

        let mut mgr = IblManager::new(&device, IblQuality::Low).expect("manager creation failed");

        // Bake environment (should use procedural sky by default)
        let resources = mgr.bake_environment(&device, &queue, IblQuality::Low);
        assert!(resources.is_ok(), "Environment bake should succeed");

        let res = resources.unwrap();
        assert_eq!(
            res.mips_specular, 8,
            "Low quality should have 8 specular mips"
        ); // 128x128 -> 8 mips
    });
}

#[test]
#[cfg(feature = "ibl")]
fn test_ibl_ensure_brdf_lut() {
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

        let mut mgr = IblManager::new(&device, IblQuality::Low).expect("manager creation failed");

        // Ensure BRDF LUT (should trigger bake)
        let view = mgr.ensure_brdf_lut(&device, &queue, IblQuality::Low);
        assert!(view.is_ok(), "BRDF LUT creation should succeed");
    });
}

#[test]
#[cfg(feature = "ibl")]
fn test_ibl_ensure_irradiance() {
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

        let mut mgr = IblManager::new(&device, IblQuality::Low).expect("manager creation failed");

        // Ensure irradiance cubemap
        let view = mgr.ensure_irradiance(&device, &queue, IblQuality::Low);
        assert!(view.is_ok(), "Irradiance cubemap creation should succeed");
    });
}

#[test]
#[cfg(feature = "ibl")]
fn test_ibl_ensure_prefiltered_env() {
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

        let mut mgr = IblManager::new(&device, IblQuality::Low).expect("manager creation failed");

        // Ensure prefiltered environment
        let view = mgr.ensure_prefiltered_env(&device, &queue, IblQuality::Low);
        assert!(
            view.is_ok(),
            "Prefiltered environment creation should succeed"
        );
    });
}

#[test]
#[cfg(feature = "ibl")]
fn test_ibl_bind_group_creation() {
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

        let mut mgr = IblManager::new(&device, IblQuality::Low).expect("manager creation failed");
        let resources = mgr
            .bake_environment(&device, &queue, IblQuality::Low)
            .expect("bake failed");

        // Create bind group
        let _bg = mgr.create_bind_group(&device, &resources);
        // If we got here without panicking, bind group creation succeeded
    });
}

// Placeholder test for non-ibl feature path
#[test]
#[cfg(not(feature = "ibl"))]
fn test_ibl_disabled() {
    // When IBL feature is disabled, this test ensures the crate still compiles
    eprintln!("IBL feature disabled; runtime IBL validation is skipped.");
}
