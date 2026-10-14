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
}
