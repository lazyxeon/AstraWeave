//! Channel layout for glTF and Poly Haven surface maps.
//!
//! The forward shader samples metallic-roughness as **R = metallic, G = roughness**.
//! glTF stores roughness in G and metallic in B. Poly Haven `arm` packs
//! **R = ambient occlusion, G = roughness, B = metallic**. Poly Haven `nor_gl`
//! is OpenGL / glTF tangent space (Y up). These helpers convert those sources
//! into the engine layout. `nor_gl` is not flipped to DirectX.

/// glTF metallic-roughness (G = roughness, B = metallic) → engine (R = metallic, G = roughness).
pub fn gltf_metallic_roughness_to_engine(src: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; src.len() - src.len() % 4];
    for (s, d) in src.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = 0;
        d[3] = s[3];
    }
    out
}

/// Poly Haven ARM → `(engine metallic-roughness, occlusion)`.
///
/// ARM is R = AO, G = roughness, B = metallic. Occlusion is replicated into
/// RGB so a shader can sample `.r`.
pub fn split_polyhaven_arm(src: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let n = src.len() - src.len() % 4;
    let mut metallic_roughness = vec![0u8; n];
    let mut occlusion = vec![0u8; n];
    for (s, (mr, ao)) in src.chunks_exact(4).zip(
        metallic_roughness
            .chunks_exact_mut(4)
            .zip(occlusion.chunks_exact_mut(4)),
    ) {
        mr[0] = s[2];
        mr[1] = s[1];
        mr[2] = 0;
        mr[3] = 255;
        ao[0] = s[0];
        ao[1] = s[0];
        ao[2] = s[0];
        ao[3] = 255;
    }
    (metallic_roughness, occlusion)
}

/// `nor_gl` bytes are uploaded unchanged. The name marks the OpenGL Y-up
/// convention so a caller does not apply a DirectX Y flip on the way in.
pub fn nor_gl_unchanged(src: &[u8]) -> Vec<u8> {
    src.to_vec()
}

/// Poly Haven ARM → glTF metallic-roughness plus occlusion.
///
/// ARM is R = AO, G = roughness, B = metallic. The metallic-roughness result
/// keeps that G/B layout so [`gltf_metallic_roughness_to_engine`] runs once,
/// at upload. Occlusion is replicated into RGB.
pub fn arm_to_gltf_mr_and_occlusion(src: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let n = src.len() - src.len() % 4;
    let mut metallic_roughness = vec![0u8; n];
    let mut occlusion = vec![0u8; n];
    for (s, (mr, ao)) in src.chunks_exact(4).zip(
        metallic_roughness
            .chunks_exact_mut(4)
            .zip(occlusion.chunks_exact_mut(4)),
    ) {
        mr[0] = 0;
        mr[1] = s[1];
        mr[2] = s[2];
        mr[3] = 255;
        ao[0] = s[0];
        ao[1] = s[0];
        ao[2] = s[0];
        ao[3] = 255;
    }
    (metallic_roughness, occlusion)
}

/// Which sidecar a filename is. `nor_gl` is OpenGL Y-up. `_arm` is Poly Haven
/// ARM. Bare `arm` inside `warm` or `armor` does not match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidecarMapKind {
    NorGl,
    Arm,
}

/// Classify a Poly Haven sidecar filename. Other extensions and DirectX
/// normals (`nor_dx`) return `None`.
pub fn classify_pbr_sidecar(file_name: &str) -> Option<SidecarMapKind> {
    let lower = file_name.to_ascii_lowercase();
    let (stem, ext) = lower.rsplit_once('.')?;
    if !matches!(ext, "png" | "jpg" | "jpeg") {
        return None;
    }
    if stem.contains("nor_gl") {
        return Some(SidecarMapKind::NorGl);
    }
    if stem.contains("_arm") || stem == "arm" || stem.starts_with("arm_") {
        return Some(SidecarMapKind::Arm);
    }
    None
}

/// Engine-layout maps ready for [`crate::renderer::ModelSurfaceMaps`].
///
/// `None` means the mesh is albedo-only (or untextured) and must stay on the
/// albedo upload path. glTF metallic-roughness has already been converted.
pub struct PackedSurface {
    pub albedo: Vec<u8>,
    pub albedo_size: (u32, u32),
    pub normal: Vec<u8>,
    pub normal_size: (u32, u32),
    pub metallic_roughness: Vec<u8>,
    pub metallic_roughness_size: (u32, u32),
    pub occlusion: Option<(u32, u32, Vec<u8>)>,
    pub emissive: Option<(u32, u32, Vec<u8>)>,
}

/// Pack a glTF [`crate::mesh::CpuMesh`] for the PBR bind group.
///
/// Returns `None` when the mesh has no normal, metallic-roughness, occlusion,
/// or emissive image. Missing albedo becomes 1×1 white. Missing normal becomes
/// a flat OpenGL normal. Missing metallic-roughness becomes dielectric
/// roughness 1 in engine layout (R = 0, G = 255).
pub fn pack_gltf_surface(cpu: &crate::mesh::CpuMesh) -> Option<PackedSurface> {
    let has_surface = cpu.normal_image.is_some()
        || cpu.metallic_roughness_image.is_some()
        || cpu.occlusion_image.is_some()
        || cpu.emissive_image.is_some();
    if !has_surface {
        return None;
    }
    let (albedo_size, albedo) = match &cpu.albedo_image {
        Some(image) => ((image.width, image.height), image.pixels.clone()),
        None => ((1, 1), vec![255, 255, 255, 255]),
    };
    let (normal_size, normal) = match &cpu.normal_image {
        Some(image) => ((image.width, image.height), nor_gl_unchanged(&image.pixels)),
        None => ((1, 1), vec![128, 128, 255, 255]),
    };
    let (metallic_roughness_size, metallic_roughness) = match &cpu.metallic_roughness_image {
        Some(image) => (
            (image.width, image.height),
            gltf_metallic_roughness_to_engine(&image.pixels),
        ),
        None => ((1, 1), vec![0, 255, 0, 255]),
    };
    let occlusion = cpu
        .occlusion_image
        .as_ref()
        .map(|image| (image.width, image.height, image.pixels.clone()));
    let emissive = cpu
        .emissive_image
        .as_ref()
        .map(|image| (image.width, image.height, image.pixels.clone()));
    Some(PackedSurface {
        albedo,
        albedo_size,
        normal,
        normal_size,
        metallic_roughness,
        metallic_roughness_size,
        occlusion,
        emissive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gltf_metallic_roughness_moves_blue_to_red() {
        // pixel: R unused, G roughness 200, B metallic 40, A 255
        let engine = gltf_metallic_roughness_to_engine(&[0, 200, 40, 255]);
        assert_eq!(engine, vec![40, 200, 0, 255]);
    }

    #[test]
    fn polyhaven_arm_splits_ao_roughness_and_metal() {
        // R ao 10, G rough 80, B metal 250
        let (mr, ao) = split_polyhaven_arm(&[10, 80, 250, 255]);
        assert_eq!(mr, vec![250, 80, 0, 255]);
        assert_eq!(ao, vec![10, 10, 10, 255]);
    }

    #[test]
    fn nor_gl_keeps_green_channel() {
        let src = [128u8, 200, 255, 255];
        assert_eq!(nor_gl_unchanged(&src), src);
    }

    #[test]
    fn arm_bytes_stay_in_gltf_slots() {
        let (mr, ao) = arm_to_gltf_mr_and_occlusion(&[10, 80, 250, 255]);
        assert_eq!(mr, vec![0, 80, 250, 255]);
        assert_eq!(ao, vec![10, 10, 10, 255]);
        assert_eq!(
            gltf_metallic_roughness_to_engine(&mr),
            vec![250, 80, 0, 255]
        );
    }

    #[test]
    fn sidecar_names_match_nor_gl_and_arm_only() {
        assert_eq!(
            classify_pbr_sidecar("barrel_nor_gl_2k.jpg"),
            Some(SidecarMapKind::NorGl)
        );
        assert_eq!(
            classify_pbr_sidecar("Barrel_arm_2k.PNG"),
            Some(SidecarMapKind::Arm)
        );
        assert_eq!(classify_pbr_sidecar("arm.jpg"), Some(SidecarMapKind::Arm));
        assert_eq!(classify_pbr_sidecar("warm_diffuse.png"), None);
        assert_eq!(classify_pbr_sidecar("armor.png"), None);
        assert_eq!(classify_pbr_sidecar("barrel_nor_dx.jpg"), None);
        assert_eq!(classify_pbr_sidecar("barrel_nor_gl_2k.exr"), None);
    }

    #[test]
    fn pack_converts_gltf_mr_and_keeps_albedo_only_off_the_pbr_path() {
        use crate::mesh::{CpuImage, CpuMesh};

        let albedo_only = CpuMesh {
            albedo_image: Some(CpuImage {
                width: 1,
                height: 1,
                pixels: vec![1, 2, 3, 255],
            }),
            ..CpuMesh::default()
        };
        assert!(pack_gltf_surface(&albedo_only).is_none());

        let mapped = CpuMesh {
            metallic_roughness_image: Some(CpuImage {
                width: 1,
                height: 1,
                pixels: vec![0, 200, 40, 255],
            }),
            ..CpuMesh::default()
        };
        let packed = pack_gltf_surface(&mapped).expect("mr map takes the pbr path");
        assert_eq!(packed.albedo, vec![255, 255, 255, 255]);
        assert_eq!(packed.normal, vec![128, 128, 255, 255]);
        assert_eq!(packed.metallic_roughness, vec![40, 200, 0, 255]);
        assert!(packed.occlusion.is_none());
        assert!(packed.emissive.is_none());
    }
}
