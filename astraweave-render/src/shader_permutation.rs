//! Shader permutation system — compile-time feature elimination for WGSL.
//!
//! Instead of runtime branching (which causes warp divergence when adjacent
//! fragments use different material features), this module generates WGSL
//! `const bool` preambles that allow the GPU compiler to statically eliminate
//! dead branches at pipeline creation time.
//!
//! # Usage
//!
//! ```rust,ignore
//! use astraweave_render::shader_permutation::ShaderPermutation;
//!
//! // Base PBR only (no optional lobes)
//! let base = ShaderPermutation::NONE;
//! let shader_src = base.generate_disney_brdf();
//!
//! // Car paint: clearcoat enabled
//! let car = ShaderPermutation::CLEARCOAT;
//! let shader_src = car.generate_disney_brdf();
//!
//! // Full material: all optional lobes
//! let full = ShaderPermutation::ALL;
//! let shader_src = full.generate_disney_brdf();
//! ```

/// Shader permutation flags — each bit enables a Disney BRDF lobe.
///
/// When a flag is NOT set, the corresponding WGSL `const` is `false` and the
/// GPU compiler eliminates the branch entirely (no warp divergence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderPermutation(u32);

impl ShaderPermutation {
    /// No optional lobes — base diffuse + specular only.
    pub const NONE: Self = Self(0);

    /// Enable clearcoat lobe (GGX with IOR 1.5).
    pub const CLEARCOAT: Self = Self(1 << 0);

    /// Enable anisotropic specular (rotated GGX).
    pub const ANISOTROPY: Self = Self(1 << 1);

    /// Enable subsurface scattering approximation (wrap lighting).
    pub const SUBSURFACE: Self = Self(1 << 2);

    /// Enable sheen lobe (Charlie distribution for fabric/velvet).
    pub const SHEEN: Self = Self(1 << 3);

    /// Enable transmission/refraction (Beer's law attenuation).
    pub const TRANSMISSION: Self = Self(1 << 4);

    /// All optional lobes enabled. Equivalent to the current runtime-branching path.
    pub const ALL: Self = Self(0x1F);

    /// Create from raw flags.
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits & 0x1F)
    }

    /// Get raw flag bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Check if a specific flag is set.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine two permutations.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Generate the WGSL const-defines preamble for this permutation.
    ///
    /// Produces lines like:
    /// ```wgsl
    /// const ENABLE_CLEARCOAT: bool = true;
    /// const ENABLE_ANISOTROPY: bool = false;
    /// ```
    ///
    /// The GPU compiler statically eliminates branches gated on `false` consts,
    /// avoiding warp divergence entirely.
    pub fn generate_defines(&self) -> String {
        format!(
            "// Auto-generated shader permutation defines (flags=0x{:02X})\n\
             const ENABLE_CLEARCOAT: bool = {};\n\
             const ENABLE_ANISOTROPY: bool = {};\n\
             const ENABLE_SUBSURFACE: bool = {};\n\
             const ENABLE_SHEEN: bool = {};\n\
             const ENABLE_TRANSMISSION: bool = {};\n\n",
            self.0,
            self.contains(Self::CLEARCOAT),
            self.contains(Self::ANISOTROPY),
            self.contains(Self::SUBSURFACE),
            self.contains(Self::SHEEN),
            self.contains(Self::TRANSMISSION),
        )
    }

    /// Generate a complete Disney BRDF shader source with permutation defines.
    ///
    /// The result includes `constants.wgsl` + permutation defines + `disney_brdf.wgsl`.
    /// Unused lobes are compiled out by the GPU compiler.
    pub fn generate_disney_brdf(&self) -> String {
        let constants = include_str!("../shaders/constants.wgsl");
        let defines = self.generate_defines();
        let brdf = include_str!("../shaders/pbr/disney_brdf.wgsl");
        format!("{constants}\n{defines}{brdf}")
    }

    /// Return a human-readable label for this permutation (for debug/logging).
    pub fn label(&self) -> String {
        if self.0 == 0 {
            return "base".to_string();
        }
        let mut parts = Vec::new();
        if self.contains(Self::CLEARCOAT) {
            parts.push("clearcoat");
        }
        if self.contains(Self::ANISOTROPY) {
            parts.push("aniso");
        }
        if self.contains(Self::SUBSURFACE) {
            parts.push("sss");
        }
        if self.contains(Self::SHEEN) {
            parts.push("sheen");
        }
        if self.contains(Self::TRANSMISSION) {
            parts.push("transmission");
        }
        parts.join("+")
    }
}

/// Common permutation presets for typical material classes.
impl ShaderPermutation {
    /// Opaque dielectric — base diffuse + specular only. Most common.
    pub const OPAQUE_DIELECTRIC: Self = Self::NONE;

    /// Metal — base diffuse + specular only (metallic flag is per-material, not per-permutation).
    pub const METAL: Self = Self::NONE;

    /// Car paint — clearcoat (fixed IOR 1.5).
    pub const CAR_PAINT: Self = Self::CLEARCOAT;

    /// Fabric — sheen lobe (Charlie distribution).
    pub const FABRIC: Self = Self::SHEEN;

    /// Skin — subsurface approximation.
    pub const SKIN: Self = Self(Self::SUBSURFACE.0);

    /// Glass — transmission + optional clearcoat.
    pub const GLASS: Self = Self(Self::TRANSMISSION.0 | Self::CLEARCOAT.0);

    /// Brushed metal — anisotropic specular.
    pub const BRUSHED_METAL: Self = Self::ANISOTROPY;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_has_no_flags() {
        assert_eq!(ShaderPermutation::NONE.bits(), 0);
        assert!(!ShaderPermutation::NONE.contains(ShaderPermutation::CLEARCOAT));
    }

    #[test]
    fn all_has_all_flags() {
        let all = ShaderPermutation::ALL;
        assert!(all.contains(ShaderPermutation::CLEARCOAT));
        assert!(all.contains(ShaderPermutation::ANISOTROPY));
        assert!(all.contains(ShaderPermutation::SUBSURFACE));
        assert!(all.contains(ShaderPermutation::SHEEN));
        assert!(all.contains(ShaderPermutation::TRANSMISSION));
    }

    #[test]
    fn union_combines_flags() {
        let combined = ShaderPermutation::CLEARCOAT.union(ShaderPermutation::SHEEN);
        assert!(combined.contains(ShaderPermutation::CLEARCOAT));
        assert!(combined.contains(ShaderPermutation::SHEEN));
        assert!(!combined.contains(ShaderPermutation::ANISOTROPY));
    }

    #[test]
    fn defines_preamble_format() {
        let defines = ShaderPermutation::CLEARCOAT.generate_defines();
        assert!(defines.contains("ENABLE_CLEARCOAT: bool = true"));
        assert!(defines.contains("ENABLE_ANISOTROPY: bool = false"));
        assert!(defines.contains("ENABLE_SUBSURFACE: bool = false"));
        assert!(defines.contains("ENABLE_SHEEN: bool = false"));
        assert!(defines.contains("ENABLE_TRANSMISSION: bool = false"));
    }

    #[test]
    fn none_defines_all_false() {
        let defines = ShaderPermutation::NONE.generate_defines();
        assert!(defines.contains("ENABLE_CLEARCOAT: bool = false"));
        assert!(defines.contains("ENABLE_ANISOTROPY: bool = false"));
        assert!(defines.contains("ENABLE_SUBSURFACE: bool = false"));
        assert!(defines.contains("ENABLE_SHEEN: bool = false"));
        assert!(defines.contains("ENABLE_TRANSMISSION: bool = false"));
    }

    #[test]
    fn all_defines_all_true() {
        let defines = ShaderPermutation::ALL.generate_defines();
        assert!(defines.contains("ENABLE_CLEARCOAT: bool = true"));
        assert!(defines.contains("ENABLE_ANISOTROPY: bool = true"));
        assert!(defines.contains("ENABLE_SUBSURFACE: bool = true"));
        assert!(defines.contains("ENABLE_SHEEN: bool = true"));
        assert!(defines.contains("ENABLE_TRANSMISSION: bool = true"));
    }

    #[test]
    fn label_format() {
        assert_eq!(ShaderPermutation::NONE.label(), "base");
        assert_eq!(ShaderPermutation::CLEARCOAT.label(), "clearcoat");
        assert_eq!(
            ShaderPermutation::CAR_PAINT.label(),
            "clearcoat"
        );
        let multi = ShaderPermutation::CLEARCOAT.union(ShaderPermutation::SHEEN);
        assert_eq!(multi.label(), "clearcoat+sheen");
    }

    #[test]
    fn presets_correct_flags() {
        assert_eq!(ShaderPermutation::OPAQUE_DIELECTRIC, ShaderPermutation::NONE);
        assert!(ShaderPermutation::CAR_PAINT.contains(ShaderPermutation::CLEARCOAT));
        assert!(ShaderPermutation::FABRIC.contains(ShaderPermutation::SHEEN));
        assert!(ShaderPermutation::SKIN.contains(ShaderPermutation::SUBSURFACE));
        assert!(ShaderPermutation::GLASS.contains(ShaderPermutation::TRANSMISSION));
        assert!(ShaderPermutation::GLASS.contains(ShaderPermutation::CLEARCOAT));
        assert!(ShaderPermutation::BRUSHED_METAL.contains(ShaderPermutation::ANISOTROPY));
    }

    #[test]
    fn generate_disney_brdf_parses_all_variants() {
        // Test that every single-flag permutation generates valid WGSL
        let flags = [
            ShaderPermutation::NONE,
            ShaderPermutation::CLEARCOAT,
            ShaderPermutation::ANISOTROPY,
            ShaderPermutation::SUBSURFACE,
            ShaderPermutation::SHEEN,
            ShaderPermutation::TRANSMISSION,
            ShaderPermutation::ALL,
        ];
        for perm in &flags {
            let src = perm.generate_disney_brdf();
            let result = naga::front::wgsl::parse_str(&src);
            assert!(
                result.is_ok(),
                "Disney BRDF permutation {:?} failed to parse: {:?}",
                perm.label(),
                result.err()
            );
        }
    }

    #[test]
    fn from_bits_masks_invalid() {
        let p = ShaderPermutation::from_bits(0xFFFF_FFFF);
        assert_eq!(p.bits(), 0x1F);
    }

    #[test]
    fn compute_shaders_have_override_workgroup_sizes() {
        // Validate that key compute shaders use override workgroup sizes
        // for runtime tuning via PipelineCompilationOptions::constants.
        let shaders_2d: &[(&str, &str)] = &[
            ("bloom_downsample", include_str!("../shaders/bloom_downsample.wgsl")),
            ("bloom_upsample", include_str!("../shaders/bloom_upsample.wgsl")),
            ("hiz_pyramid", include_str!("../shaders/hiz_pyramid.wgsl")),
            ("ssr", include_str!("../shaders/ssr.wgsl")),
            ("ssgi", include_str!("../shaders/ssgi.wgsl")),
            ("compute_noise", include_str!("../shaders/compute_noise.wgsl")),
            ("snow_accumulation", include_str!("../shaders/snow_accumulation.wgsl")),
            ("gpu_erosion", include_str!("../shaders/gpu_erosion.wgsl")),
            ("virtual_texture", include_str!("../shaders/virtual_texture.wgsl")),
        ];
        for (name, src) in shaders_2d {
            assert!(
                src.contains("override WG_X: u32"),
                "{name} missing override WG_X"
            );
            assert!(
                src.contains("override WG_Y: u32"),
                "{name} missing override WG_Y"
            );
            assert!(
                src.contains("@workgroup_size(WG_X, WG_Y"),
                "{name} not using override in @workgroup_size"
            );
        }

        let shaders_1d: &[(&str, &str)] = &[
            ("vegetation_scatter", include_str!("../shaders/vegetation_scatter.wgsl")),
            ("simulate", include_str!("../shaders/particles/simulate.wgsl")),
            ("rain_occlusion", include_str!("../shaders/particles/rain_occlusion.wgsl")),
        ];
        for (name, src) in shaders_1d {
            assert!(
                src.contains("override WG_SIZE: u32"),
                "{name} missing override WG_SIZE"
            );
            assert!(
                src.contains("@workgroup_size(WG_SIZE)"),
                "{name} not using override in @workgroup_size"
            );
        }

        let shaders_3d: &[(&str, &str)] = &[
            ("scatter", include_str!("../shaders/volumetrics/scatter.wgsl")),
            ("fog_density", include_str!("../shaders/volumetrics/fog_density.wgsl")),
        ];
        for (name, src) in shaders_3d {
            assert!(
                src.contains("override WG_X: u32"),
                "{name} missing override WG_X"
            );
            assert!(
                src.contains("override WG_Z: u32"),
                "{name} missing override WG_Z"
            );
            assert!(
                src.contains("@workgroup_size(WG_X, WG_Y, WG_Z)"),
                "{name} not using override in @workgroup_size"
            );
        }
    }
}
