//! Subgroup operation support for GPU compute shaders.
//!
//! Provides capability detection and shader variant selection for subgroup-optimized
//! compute passes (auto-exposure, prefix sum, bitonic sort).
//!
//! Subgroup operations (`subgroupAdd`, `subgroupExclusiveAdd`, `subgroupShuffleXor`)
//! collapse `log2(subgroup_size)` tree-reduction steps into single instructions,
//! eliminating `workgroupBarrier()` calls and global memory round-trips.
//!
//! # Feature Gating
//!
//! Subgroup operations require `wgpu::Features::SUBGROUP` (Vulkan/Metal only — not WebGPU).
//! All subgroup shaders have non-subgroup fallbacks; the renderer selects the variant at
//! pipeline creation time based on adapter capabilities.
//!
//! # Optimized Shaders
//!
//! | Shader | Subgroup Optimization | Estimated Speedup |
//! |--------|----------------------|-------------------|
//! | Auto-exposure (average pass) | `subgroupAdd` reduction | 2–3× fewer barriers |
//! | MegaLights prefix sum | `subgroupExclusiveAdd` scan | ~30% fewer barriers |
//! | Bitonic sort (inner stages) | `subgroupShuffleXor` swap | ~30–40% fewer dispatches |

/// WGSL source for the subgroup-optimized auto-exposure shader.
pub const AUTO_EXPOSURE_SUBGROUP_WGSL: &str =
    include_str!("../shaders/subgroup/auto_exposure_subgroup.wgsl");

/// WGSL source for the subgroup-optimized prefix sum shader.
pub const PREFIX_SUM_SUBGROUP_WGSL: &str =
    include_str!("../shaders/subgroup/prefix_sum_subgroup.wgsl");

/// WGSL source for the subgroup-optimized bitonic sort shader.
pub const BITONIC_SORT_SUBGROUP_WGSL: &str =
    include_str!("../shaders/subgroup/bitonic_sort_subgroup.wgsl");

/// Subgroup capability information for the current device.
#[derive(Debug, Clone, Default)]
pub struct SubgroupCapabilities {
    /// Whether the device supports subgroup operations at all.
    pub supported: bool,
    /// Minimum subgroup size (typically 4 on Intel, 32 on NVIDIA, 64 on AMD).
    pub min_subgroup_size: u32,
    /// Maximum subgroup size.
    pub max_subgroup_size: u32,
}

impl SubgroupCapabilities {
    /// Query subgroup capabilities from wgpu adapter features.
    ///
    /// Returns capabilities with `supported = true` only if the adapter
    /// advertises `wgpu::Features::SUBGROUP`.
    pub fn from_features(features: wgpu::Features) -> Self {
        let supported = features.contains(wgpu::Features::SUBGROUP);
        if supported {
            // wgpu doesn't expose subgroup size directly in Features;
            // actual size is determined at shader execution time via
            // @builtin(subgroup_size). We report common GPU ranges.
            Self {
                supported: true,
                min_subgroup_size: 4,  // Intel iGPU min
                max_subgroup_size: 64, // AMD RDNA max
            }
        } else {
            Self {
                supported: false,
                min_subgroup_size: 0,
                max_subgroup_size: 0,
            }
        }
    }

    /// Check if subgroup operations are available.
    pub fn is_available(&self) -> bool {
        self.supported
    }
}

/// Select the appropriate shader source based on subgroup capabilities.
///
/// Returns `(shader_source, is_subgroup_variant)`.
pub fn select_auto_exposure_shader(caps: &SubgroupCapabilities) -> (&'static str, bool) {
    if caps.is_available() {
        (AUTO_EXPOSURE_SUBGROUP_WGSL, true)
    } else {
        (include_str!("../shaders/auto_exposure.wgsl"), false)
    }
}

/// Select the appropriate prefix sum shader source.
pub fn select_prefix_sum_shader(caps: &SubgroupCapabilities) -> (&'static str, bool) {
    if caps.is_available() {
        (PREFIX_SUM_SUBGROUP_WGSL, true)
    } else {
        (include_str!("../shaders/megalights/prefix_sum.wgsl"), false)
    }
}

/// Select the appropriate bitonic sort shader source.
pub fn select_bitonic_sort_shader(caps: &SubgroupCapabilities) -> (&'static str, bool) {
    if caps.is_available() {
        (BITONIC_SORT_SUBGROUP_WGSL, true)
    } else {
        (
            include_str!("../shaders/particles/bitonic_sort.wgsl"),
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_default_no_subgroup() {
        let caps = SubgroupCapabilities::default();
        assert!(!caps.supported);
        assert!(!caps.is_available());
        assert_eq!(caps.min_subgroup_size, 0);
    }

    #[test]
    fn capabilities_from_empty_features() {
        let caps = SubgroupCapabilities::from_features(wgpu::Features::empty());
        assert!(!caps.supported);
    }

    #[test]
    fn capabilities_from_subgroup_feature() {
        let caps = SubgroupCapabilities::from_features(wgpu::Features::SUBGROUP);
        assert!(caps.supported);
        assert!(caps.is_available());
        assert!(caps.min_subgroup_size > 0);
        assert!(caps.max_subgroup_size >= caps.min_subgroup_size);
    }

    #[test]
    fn select_auto_exposure_without_subgroup() {
        let caps = SubgroupCapabilities::default();
        let (source, is_subgroup) = select_auto_exposure_shader(&caps);
        assert!(!is_subgroup);
        assert!(source.contains("histogram_pass"));
        assert!(!source.contains("enable subgroups"));
    }

    #[test]
    fn select_auto_exposure_with_subgroup() {
        let caps = SubgroupCapabilities::from_features(wgpu::Features::SUBGROUP);
        let (source, is_subgroup) = select_auto_exposure_shader(&caps);
        assert!(is_subgroup);
        assert!(source.contains("enable subgroups"));
        assert!(source.contains("subgroupAdd"));
    }

    #[test]
    fn select_prefix_sum_without_subgroup() {
        let caps = SubgroupCapabilities::default();
        let (source, is_subgroup) = select_prefix_sum_shader(&caps);
        assert!(!is_subgroup);
        assert!(source.contains("prefix_sum"));
    }

    #[test]
    fn select_prefix_sum_with_subgroup() {
        let caps = SubgroupCapabilities::from_features(wgpu::Features::SUBGROUP);
        let (source, is_subgroup) = select_prefix_sum_shader(&caps);
        assert!(is_subgroup);
        assert!(source.contains("subgroupExclusiveAdd"));
    }

    #[test]
    fn select_bitonic_sort_without_subgroup() {
        let caps = SubgroupCapabilities::default();
        let (source, is_subgroup) = select_bitonic_sort_shader(&caps);
        assert!(!is_subgroup);
        assert!(source.contains("bitonic_sort"));
    }

    #[test]
    fn select_bitonic_sort_with_subgroup() {
        let caps = SubgroupCapabilities::from_features(wgpu::Features::SUBGROUP);
        let (source, is_subgroup) = select_bitonic_sort_shader(&caps);
        assert!(is_subgroup);
        assert!(source.contains("subgroupShuffleXor"));
    }

    #[test]
    fn subgroup_shader_sources_are_nonempty() {
        assert!(!AUTO_EXPOSURE_SUBGROUP_WGSL.is_empty());
        assert!(!PREFIX_SUM_SUBGROUP_WGSL.is_empty());
        assert!(!BITONIC_SORT_SUBGROUP_WGSL.is_empty());
    }

    #[test]
    fn all_subgroup_shaders_have_enable_directive() {
        for (name, source) in [
            ("auto_exposure", AUTO_EXPOSURE_SUBGROUP_WGSL),
            ("prefix_sum", PREFIX_SUM_SUBGROUP_WGSL),
            ("bitonic_sort", BITONIC_SORT_SUBGROUP_WGSL),
        ] {
            assert!(
                source.contains("enable subgroups;"),
                "{name} missing 'enable subgroups;' directive"
            );
        }
    }

    #[test]
    fn all_subgroup_shaders_have_compute_entry_point() {
        for (name, source) in [
            ("auto_exposure", AUTO_EXPOSURE_SUBGROUP_WGSL),
            ("prefix_sum", PREFIX_SUM_SUBGROUP_WGSL),
            ("bitonic_sort", BITONIC_SORT_SUBGROUP_WGSL),
        ] {
            assert!(
                source.contains("@compute"),
                "{name} missing @compute entry point"
            );
        }
    }

    // Note: naga parse tests are intentionally omitted for subgroup shaders because
    // naga's WGSL frontend may not support `enable subgroups;` / subgroup builtins.
    // These shaders are validated by wgpu at pipeline creation time on capable hardware.
}
