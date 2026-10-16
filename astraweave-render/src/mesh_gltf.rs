use crate::mesh::CpuMesh;
#[cfg(feature = "gltf-assets")]
use crate::mesh::{compute_tangents, CpuImage, MeshVertex};
use anyhow::{anyhow, Result};

pub struct GltfOptions {
    pub generate_tangents: bool,
}

impl Default for GltfOptions {
    fn default() -> Self {
        Self {
            generate_tangents: true,
        }
    }
}

#[cfg(feature = "gltf-assets")]
fn jail_gltf_uri(base: &std::path::Path, uri: &str) -> Result<()> {
    if uri.starts_with("data:") {
        return Ok(());
    }
    let path_part = uri.split('?').next().unwrap_or(uri);
    astraweave_security::path::safe_under(base, std::path::Path::new(path_part))
        .map_err(|err| anyhow!("path traversal rejected: {err}"))?;
    Ok(())
}

#[cfg(feature = "gltf-assets")]
fn reject_escaping_gltf_uris(path: &std::path::Path) -> Result<()> {
    let base = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let doc = gltf::Gltf::open(path)?;
    for buffer in doc.buffers() {
        if let gltf::buffer::Source::Uri(uri) = buffer.source() {
            jail_gltf_uri(base, uri)?;
        }
    }
    for image in doc.images() {
        if let gltf::image::Source::Uri { uri, .. } = image.source() {
            jail_gltf_uri(base, uri)?;
        }
    }
    Ok(())
}

#[cfg(feature = "gltf-assets")]
pub fn load_gltf(path: &std::path::Path, opts: &GltfOptions) -> Result<Vec<CpuMesh>> {
    use gltf::mesh::util::ReadIndices;
    reject_escaping_gltf_uris(path)?;
    let (doc, buffers, images) = gltf::import(path)?;
    let mut meshes_out = Vec::new();
    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| Some(&buffers[b.index()]));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| anyhow!("missing POSITION"))?
                .collect();
            let normals: Vec<[f32; 3]> = match reader.read_normals() {
                Some(iter) => iter.collect(),
                None => positions.iter().map(|_| [0.0f32, 1.0, 0.0]).collect(),
            };
            let tangents_opt: Option<Vec<[f32; 4]>> = reader.read_tangents().map(|i| i.collect());
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|tc| tc.into_f32().collect())
                .unwrap_or_else(|| positions.iter().map(|_| [0.0f32, 0.0]).collect());
            let indices: Vec<u32> = match reader.read_indices() {
                Some(ReadIndices::U16(i)) => i.map(|x| x as u32).collect(),
                Some(ReadIndices::U32(i)) => i.collect(),
                Some(ReadIndices::U8(i)) => i.map(|x| x as u32).collect(),
                None => (0..positions.len() as u32).collect(),
            };

            let mut cpu = CpuMesh {
                vertices: Vec::with_capacity(positions.len()),
                indices,
                ..CpuMesh::default()
            };
            if let Some(t) = tangents_opt {
                for ((p, n), (u, t4)) in positions
                    .iter()
                    .zip(normals.iter())
                    .zip(uvs.iter().zip(t.iter()))
                {
                    cpu.vertices.push(MeshVertex::from_arrays(*p, *n, *t4, *u));
                }
            } else {
                // Fill with placeholder tangent; can be generated later
                for ((p, n), u) in positions.iter().zip(normals.iter()).zip(uvs.iter()) {
                    cpu.vertices
                        .push(MeshVertex::from_arrays(*p, *n, [1.0, 0.0, 0.0, 1.0], *u));
                }
                if opts.generate_tangents {
                    generate_mikktspace_tangents(&mut cpu)?;
                }
            }
            let material = prim.material();
            let pbr = material.pbr_metallic_roughness();
            cpu.albedo_image = image_from_texture(&images, pbr.base_color_texture())?;
            cpu.metallic_roughness_image =
                image_from_texture(&images, pbr.metallic_roughness_texture())?;
            cpu.normal_image = image_from_texture(&images, material.normal_texture())?;
            cpu.occlusion_image = image_from_texture(&images, material.occlusion_texture())?;
            cpu.emissive_image = image_from_texture(&images, material.emissive_texture())?;

            // Extract texture source hint for filesystem-based discovery.
            // Even when the image data isn't embedded/loadable, the glTF
            // material often references a URI like "../textures/bark_diff.png"
            // which tells us which texture file to look for on disk.
            cpu.texture_source_hint = prim
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
                .and_then(|info| {
                    let src = info.texture().source().source();
                    match src {
                        gltf::image::Source::Uri { uri, .. } => {
                            // Extract filename stem from URI path
                            let path = std::path::Path::new(uri);
                            path.file_stem().and_then(|s| s.to_str()).map(|s| {
                                // Strip double-extension (.png.png → .png stem)
                                let p = std::path::Path::new(s);
                                p.file_stem()
                                    .and_then(|ss| ss.to_str())
                                    .unwrap_or(s)
                                    .to_string()
                            })
                        }
                        gltf::image::Source::View { .. } => None,
                    }
                })
                .or_else(|| {
                    // Fallback: use glTF material name as hint
                    prim.material().name().map(|n| n.to_string())
                });

            meshes_out.push(cpu);
        }
    }
    Ok(meshes_out)
}

#[cfg(all(test, feature = "gltf-assets"))]
mod jail_tests {
    use super::*;

    #[test]
    fn gltf_buffer_parent_uri_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("hostile.gltf");
        std::fs::write(
            &path,
            r#"{
                "asset": {"version": "2.0"},
                "buffers": [{"uri": "../secret.bin", "byteLength": 4}]
            }"#,
        )
        .expect("write gltf");
        let err = load_gltf(&path, &GltfOptions::default()).expect_err("hostile gltf");
        let msg = format!("{err:#}").to_lowercase();
        assert!(
            msg.contains("traversal") || msg.contains("rejected"),
            "{msg}"
        );
    }
}

#[cfg(feature = "gltf-assets")]
fn image_from_texture<'a, T>(
    images: &'a [gltf::image::Data],
    info: Option<T>,
) -> Result<Option<CpuImage>>
where
    T: TextureInfo,
{
    let Some(info) = info else {
        return Ok(None);
    };
    let idx = info.texture_index();
    let Some(img) = images.get(idx) else {
        return Ok(None);
    };
    Ok(Some(convert_gltf_image(img)?))
}

#[cfg(feature = "gltf-assets")]
trait TextureInfo {
    fn texture_index(&self) -> usize;
}

#[cfg(feature = "gltf-assets")]
impl TextureInfo for gltf::texture::Info<'_> {
    fn texture_index(&self) -> usize {
        self.texture().source().index()
    }
}

#[cfg(feature = "gltf-assets")]
impl<'a> TextureInfo for gltf::material::NormalTexture<'a> {
    fn texture_index(&self) -> usize {
        self.texture().source().index()
    }
}

#[cfg(feature = "gltf-assets")]
impl<'a> TextureInfo for gltf::material::OcclusionTexture<'a> {
    fn texture_index(&self) -> usize {
        self.texture().source().index()
    }
}

#[cfg(feature = "gltf-assets")]
fn convert_gltf_image(img: &gltf::image::Data) -> Result<CpuImage> {
    let pixels = match img.format {
        gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
        gltf::image::Format::R8G8B8 => {
            let mut rgba = Vec::with_capacity(img.pixels.len() / 3 * 4);
            for chunk in img.pixels.chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            rgba
        }
        gltf::image::Format::R8 => img.pixels.iter().flat_map(|&r| [r, r, r, 255]).collect(),
        gltf::image::Format::R8G8 => img
            .pixels
            .chunks_exact(2)
            .flat_map(|rg| [rg[0], rg[1], 0, 255])
            .collect(),
        gltf::image::Format::R16G16B16A16 => decode_u16_rgba(&img.pixels)?,
        gltf::image::Format::R16G16B16 => decode_u16_rgb(&img.pixels)?,
        gltf::image::Format::R32G32B32FLOAT => decode_f32_rgb(&img.pixels)?,
        gltf::image::Format::R32G32B32A32FLOAT => decode_f32_rgba(&img.pixels)?,
        other => {
            anyhow::bail!(
                "unsupported glTF image format {other:?} ({}x{}); refusing a 1x1 white stand-in",
                img.width,
                img.height
            );
        }
    };
    let expected = (img.width as usize)
        .saturating_mul(img.height as usize)
        .saturating_mul(4);
    if pixels.len() != expected {
        anyhow::bail!(
            "glTF image {}x{} decoded to {} bytes, expected {expected}",
            img.width,
            img.height,
            pixels.len()
        );
    }
    Ok(CpuImage {
        width: img.width,
        height: img.height,
        pixels,
    })
}

#[cfg(feature = "gltf-assets")]
fn u8_from_unit(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(feature = "gltf-assets")]
fn le_f32(bytes: &[u8]) -> f32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(bytes);
    f32::from_le_bytes(buf)
}

#[cfg(feature = "gltf-assets")]
fn le_u16(bytes: &[u8]) -> u16 {
    let mut buf = [0u8; 2];
    buf.copy_from_slice(bytes);
    u16::from_le_bytes(buf)
}

#[cfg(feature = "gltf-assets")]
fn decode_f32_rgb(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() % 12 != 0 {
        anyhow::bail!("truncated R32G32B32FLOAT image");
    }
    let mut rgba = Vec::with_capacity(bytes.len() / 12 * 4);
    for chunk in bytes.chunks_exact(12) {
        let r = le_f32(&chunk[0..4]);
        let g = le_f32(&chunk[4..8]);
        let b = le_f32(&chunk[8..12]);
        rgba.extend_from_slice(&[u8_from_unit(r), u8_from_unit(g), u8_from_unit(b), 255]);
    }
    Ok(rgba)
}

#[cfg(feature = "gltf-assets")]
fn decode_f32_rgba(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() % 16 != 0 {
        anyhow::bail!("truncated R32G32B32A32FLOAT image");
    }
    let mut rgba = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(16) {
        for i in 0..4 {
            let v = le_f32(&chunk[i * 4..i * 4 + 4]);
            rgba.push(u8_from_unit(v));
        }
    }
    Ok(rgba)
}

#[cfg(feature = "gltf-assets")]
fn decode_u16_rgb(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() % 6 != 0 {
        anyhow::bail!("truncated R16G16B16 image");
    }
    let mut rgba = Vec::with_capacity(bytes.len() / 6 * 4);
    for chunk in bytes.chunks_exact(6) {
        for i in 0..3 {
            let v = le_u16(&chunk[i * 2..i * 2 + 2]);
            rgba.push((v >> 8) as u8);
        }
        rgba.push(255);
    }
    Ok(rgba)
}

#[cfg(feature = "gltf-assets")]
fn decode_u16_rgba(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() % 8 != 0 {
        anyhow::bail!("truncated R16G16B16A16 image");
    }
    let mut rgba = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let v = le_u16(chunk);
        rgba.push((v >> 8) as u8);
    }
    Ok(rgba)
}

#[cfg(not(feature = "gltf-assets"))]
pub fn load_gltf(_path: &std::path::Path, _opts: &GltfOptions) -> Result<Vec<CpuMesh>> {
    Err(anyhow!("gltf-assets feature not enabled"))
}

#[cfg(feature = "gltf-assets")]
pub fn generate_mikktspace_tangents(mesh: &mut CpuMesh) -> Result<()> {
    compute_tangents(mesh);
    Ok(())
}

#[cfg(not(feature = "gltf-assets"))]
pub fn generate_mikktspace_tangents(_mesh: &mut CpuMesh) -> Result<()> {
    Ok(())
}
