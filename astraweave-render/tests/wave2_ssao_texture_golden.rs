//! Wave 2 – Golden-value tests for ssao.rs (68 mutants) + texture.rs (81 mutants)
//!
//! Targets: SsaoQuality enum method returns, SsaoConfig defaults,
//!          SsaoKernel::generate hemisphere sampling properties,
//!          TextureUsage format/mipmaps/description methods.
//!
//! Strategy: Pin exact enum→value mappings so any match arm swap or
//! arithmetic mutation is caught.

// ============================================================================
// SSAO tests – feature-gated behind "ssao"
// ============================================================================
#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[cfg(feature = "ssao")]
mod ssao_tests {
    use astraweave_render::ssao::{SsaoConfig, SsaoKernel, SsaoQuality};

    // === SsaoQuality — sample_count golden ===

    #[test]
    fn quality_low_sample_count() {
        assert_eq!(SsaoQuality::Low.sample_count(), 8);
    }

    #[test]
    fn quality_medium_sample_count() {
        assert_eq!(SsaoQuality::Medium.sample_count(), 16);
    }

    #[test]
    fn quality_high_sample_count() {
        assert_eq!(SsaoQuality::High.sample_count(), 32);
    }

    #[test]
    fn quality_ultra_sample_count() {
        assert_eq!(SsaoQuality::Ultra.sample_count(), 64);
    }

    // === SsaoQuality — radius golden ===

    #[test]
    fn quality_low_radius() {
        assert_eq!(SsaoQuality::Low.radius(), 0.5);
    }

    #[test]
    fn quality_medium_radius() {
        assert_eq!(SsaoQuality::Medium.radius(), 1.0);
    }

    #[test]
    fn quality_high_radius() {
        assert_eq!(SsaoQuality::High.radius(), 1.5);
    }

    #[test]
    fn quality_ultra_radius() {
        assert_eq!(SsaoQuality::Ultra.radius(), 2.0);
    }

    // === SsaoQuality — blur_kernel_size golden ===

    #[test]
    fn quality_low_blur_none() {
        assert_eq!(SsaoQuality::Low.blur_kernel_size(), 0);
    }

    #[test]
    fn quality_medium_blur_3() {
        assert_eq!(SsaoQuality::Medium.blur_kernel_size(), 3);
    }

    #[test]
    fn quality_high_blur_5() {
        assert_eq!(SsaoQuality::High.blur_kernel_size(), 5);
    }

    #[test]
    fn quality_ultra_blur_7() {
        assert_eq!(SsaoQuality::Ultra.blur_kernel_size(), 7);
    }

    // === SsaoQuality — ordering invariants ===

    #[test]
    fn quality_radius_increases_with_quality() {
        let qualities = [
            SsaoQuality::Low,
            SsaoQuality::Medium,
            SsaoQuality::High,
            SsaoQuality::Ultra,
        ];
        for w in qualities.windows(2) {
            assert!(
                w[1].radius() > w[0].radius(),
                "{:?} radius should exceed {:?}",
                w[1],
                w[0]
            );
        }
    }

    #[test]
    fn quality_sample_count_increases_with_quality() {
        let qualities = [
            SsaoQuality::Low,
            SsaoQuality::Medium,
            SsaoQuality::High,
            SsaoQuality::Ultra,
        ];
        for w in qualities.windows(2) {
            assert!(w[1].sample_count() > w[0].sample_count());
        }
    }

    #[test]
    fn quality_blur_increases_with_quality() {
        let qualities = [
            SsaoQuality::Low,
            SsaoQuality::Medium,
            SsaoQuality::High,
            SsaoQuality::Ultra,
        ];
        for w in qualities.windows(2) {
            assert!(w[1].blur_kernel_size() >= w[0].blur_kernel_size());
        }
    }

    // === SsaoConfig — defaults ===

    #[test]
    fn ssao_config_defaults() {
        let c = SsaoConfig::default();
        assert_eq!(c.quality, SsaoQuality::Medium);
        assert_eq!(c.radius, 1.0);
        assert_eq!(c.bias, 0.025);
        assert_eq!(c.intensity, 1.0);
        assert!(c.enabled);
    }

    // === SsaoKernel — generate hemisphere samples ===

    #[test]
    fn kernel_generates_64_samples() {
        let k = SsaoKernel::generate(64);
        for i in 0..64 {
            let s = k.samples[i];
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            assert!(len > 0.0, "Sample {} should be non-zero", i);
        }
    }

    #[test]
    fn kernel_8_samples_leaves_rest_zeroed() {
        let k = SsaoKernel::generate(8);
        for i in 0..8 {
            let s = k.samples[i];
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            assert!(len > 0.0, "Sample {} should be non-zero", i);
        }
        for i in 8..64 {
            assert_eq!(
                k.samples[i],
                [0.0, 0.0, 0.0, 0.0],
                "Sample {} should be zero",
                i
            );
        }
    }

    #[test]
    fn kernel_samples_in_upper_hemisphere() {
        let k = SsaoKernel::generate(32);
        for i in 0..32 {
            assert!(
                k.samples[i][2] >= 0.0,
                "Sample {} z={} should be >= 0 (upper hemisphere)",
                i,
                k.samples[i][2]
            );
        }
    }

    #[test]
    fn kernel_samples_fourth_component_zero() {
        let k = SsaoKernel::generate(16);
        for i in 0..16 {
            assert_eq!(k.samples[i][3], 0.0, "Sample {} w should be 0", i);
        }
    }

    #[test]
    fn kernel_samples_scale_increases() {
        let k = SsaoKernel::generate(32);
        let first_len = {
            let s = k.samples[0];
            (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
        };
        let last_len = {
            let s = k.samples[31];
            (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
        };
        assert!(
            last_len > first_len,
            "Last sample magnitude {} should exceed first {}",
            last_len,
            first_len
        );
    }

    #[test]
    fn kernel_first_sample_near_origin() {
        // i=0: scale = 0.1 + (1/32)^2 * 0.9 ≈ 0.1009
        let k = SsaoKernel::generate(32);
        let s = k.samples[0];
        let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        assert!(len < 0.2, "First sample should be near origin, len={}", len);
    }

    #[test]
    fn kernel_deterministic() {
        let k1 = SsaoKernel::generate(16);
        let k2 = SsaoKernel::generate(16);
        for i in 0..16 {
            assert_eq!(
                k1.samples[i], k2.samples[i],
                "Kernel should be deterministic at {}",
                i
            );
        }
    }
}

// ============================================================================
// TextureUsage — format golden (no feature gate needed)
// ============================================================================

use astraweave_render::texture::TextureUsage;

#[test]
fn texture_albedo_srgb_format() {
    assert_eq!(
        TextureUsage::Albedo.format(),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
}

#[test]
fn texture_emissive_srgb_format() {
    assert_eq!(
        TextureUsage::Emissive.format(),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
}

#[test]
fn texture_normal_linear_format() {
    assert_eq!(
        TextureUsage::Normal.format(),
        wgpu::TextureFormat::Rgba8Unorm
    );
}

#[test]
fn texture_mra_linear_format() {
    assert_eq!(TextureUsage::MRA.format(), wgpu::TextureFormat::Rgba8Unorm);
}

#[test]
fn texture_height_linear_format() {
    assert_eq!(
        TextureUsage::Height.format(),
        wgpu::TextureFormat::Rgba8Unorm
    );
}

// ============================================================================
// TextureUsage — needs_mipmaps golden
// ============================================================================

#[test]
fn albedo_needs_mipmaps() {
    assert!(TextureUsage::Albedo.needs_mipmaps());
}

#[test]
fn emissive_needs_mipmaps() {
    assert!(TextureUsage::Emissive.needs_mipmaps());
}

#[test]
fn mra_needs_mipmaps() {
    assert!(TextureUsage::MRA.needs_mipmaps());
}

#[test]
fn normal_no_mipmaps() {
    assert!(!TextureUsage::Normal.needs_mipmaps());
}

#[test]
fn height_no_mipmaps() {
    assert!(!TextureUsage::Height.needs_mipmaps());
}

// ============================================================================
// TextureUsage — description golden
// ============================================================================

#[test]
fn albedo_description() {
    assert_eq!(TextureUsage::Albedo.description(), "Albedo (sRGB color)");
}

#[test]
fn normal_description() {
    assert_eq!(
        TextureUsage::Normal.description(),
        "Normal Map (linear RGB)"
    );
}

#[test]
fn mra_description() {
    assert_eq!(
        TextureUsage::MRA.description(),
        "Metallic/Roughness/AO (linear)"
    );
}

#[test]
fn emissive_description() {
    assert_eq!(
        TextureUsage::Emissive.description(),
        "Emissive (sRGB color)"
    );
}

#[test]
fn height_description() {
    assert_eq!(
        TextureUsage::Height.description(),
        "Height/Displacement (linear)"
    );
}
