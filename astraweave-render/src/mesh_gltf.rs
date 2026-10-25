use crate::mesh::CpuMesh;
#[cfg(feature = "gltf-assets")]
use crate::mesh::{compute_tangents, CpuImage, MeshVertex};
use anyhow::{anyhow, Context, Result};

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
fn is_data_uri(uri: &str) -> bool {
    uri.split_once(':')
        .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("data"))
}

/// Percent-decode, reject a scheme, and return the path `safe_under` checked.
#[cfg(feature = "gltf-assets")]
fn jailed_gltf_file(base: &std::path::Path, uri: &str) -> Result<std::path::PathBuf> {
    let relative = astraweave_asset::gltf_relative_uri(uri)?;
    astraweave_security::path::safe_under(base, std::path::Path::new(&relative))
        .map_err(|err| anyhow!("path traversal rejected: {err}"))
}

#[cfg(feature = "gltf-assets")]
fn jail_gltf_uri(base: &std::path::Path, uri: &str) -> Result<()> {
    if is_data_uri(uri) {
        return Ok(());
    }
    jailed_gltf_file(base, uri)?;
    Ok(())
}

/// Reject a GLB whose header would make `Glb::from_reader` allocate.
///
/// `from_slice` refuses a length past the slice. This check runs first so a
/// short file with `header.length == u32::MAX`, or a length under 12, fails
/// here instead of inside that allocator. JSON containers skip it.
#[cfg(feature = "gltf-assets")]
fn reject_glb_header(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 4 || &bytes[..4] != b"glTF" {
        return Ok(());
    }
    if bytes.len() < 12 {
        anyhow::bail!("glTF header is truncated");
    }
    let length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    if (length as u64) < 12 {
        anyhow::bail!("glTF header length {length} is shorter than the header");
    }
    let cap = astraweave_asset::MAX_GLTF_BUFFER;
    if length as u64 > cap {
        anyhow::bail!("glTF header length {length} exceeds {cap}");
    }
    if (length as u64) > bytes.len() as u64 {
        anyhow::bail!(
            "glTF header length {length} exceeds the {} byte file",
            bytes.len()
        );
    }
    Ok(())
}

/// Read the container once, under the buffer cap, and parse that slice.
///
/// `Gltf::from_slice` uses `Glb::from_slice`, which does not allocate
/// `header.length`. `Gltf::open` would call `Glb::from_reader` and parse a
/// second time.
#[cfg(feature = "gltf-assets")]
fn open_capped_gltf(path: &std::path::Path) -> Result<gltf::Gltf> {
    let bytes = astraweave_asset::read_capped_file(path, astraweave_asset::MAX_GLTF_BUFFER)
        .with_context(|| format!("open {}", path.display()))?;
    reject_glb_header(&bytes)?;
    gltf::Gltf::from_slice(&bytes).map_err(|err| anyhow!("open glTF: {err}"))
}

#[cfg(feature = "gltf-assets")]
fn reject_escaping_gltf_uris(doc: &gltf::Document, base: &std::path::Path) -> Result<()> {
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
fn dynamic_to_gltf_image(image: image::DynamicImage) -> gltf::image::Data {
    let width = image.width();
    let height = image.height();
    let le = |values: &[u8]| values.to_vec();
    let (pixels, format) = match &image {
        image::DynamicImage::ImageLuma8(img) => (le(img.as_raw()), gltf::image::Format::R8),
        image::DynamicImage::ImageLumaA8(img) => (le(img.as_raw()), gltf::image::Format::R8G8),
        image::DynamicImage::ImageRgb8(img) => (le(img.as_raw()), gltf::image::Format::R8G8B8),
        image::DynamicImage::ImageRgba8(img) => (le(img.as_raw()), gltf::image::Format::R8G8B8A8),
        image::DynamicImage::ImageLuma16(img) => (
            bytemuck::cast_slice(img.as_raw()).to_vec(),
            gltf::image::Format::R16,
        ),
        image::DynamicImage::ImageLumaA16(img) => (
            bytemuck::cast_slice(img.as_raw()).to_vec(),
            gltf::image::Format::R16G16,
        ),
        image::DynamicImage::ImageRgb16(img) => (
            bytemuck::cast_slice(img.as_raw()).to_vec(),
            gltf::image::Format::R16G16B16,
        ),
        image::DynamicImage::ImageRgba16(img) => (
            bytemuck::cast_slice(img.as_raw()).to_vec(),
            gltf::image::Format::R16G16B16A16,
        ),
        image::DynamicImage::ImageRgb32F(img) => (
            bytemuck::cast_slice(img.as_raw()).to_vec(),
            gltf::image::Format::R32G32B32FLOAT,
        ),
        image::DynamicImage::ImageRgba32F(img) => (
            bytemuck::cast_slice(img.as_raw()).to_vec(),
            gltf::image::Format::R32G32B32A32FLOAT,
        ),
        _ => (
            image.clone().to_rgba8().into_raw(),
            gltf::image::Format::R8G8B8A8,
        ),
    };
    gltf::image::Data {
        pixels,
        format,
        width,
        height,
    }
}

#[cfg(feature = "gltf-assets")]
fn load_capped_gltf_images(
    document: &gltf::Document,
    base: &std::path::Path,
    buffers: &[gltf::buffer::Data],
) -> Result<Vec<gltf::image::Data>> {
    let cap = astraweave_asset::image_limits::MAX_IMAGE_ALLOC;
    let mut images = Vec::new();
    for image in document.images() {
        let bytes = match image.source() {
            gltf::image::Source::Uri { uri, .. } if is_data_uri(uri) => {
                astraweave_asset::read_capped_data_image(uri)?
            }
            gltf::image::Source::Uri { uri, .. } => {
                let path = jailed_gltf_file(base, uri)?;
                astraweave_asset::read_limited_image_bytes(&path)
                    .with_context(|| format!("read glTF image {}", path.display()))?
            }
            gltf::image::Source::View { view, .. } => {
                let length = view.length() as u64;
                if length > cap {
                    anyhow::bail!("glTF image view is {length} bytes, cap is {cap}");
                }
                let parent = buffers
                    .get(view.buffer().index())
                    .ok_or_else(|| anyhow!("missing glTF buffer {}", view.buffer().index()))?;
                let begin = view.offset();
                let end = begin
                    .checked_add(view.length())
                    .ok_or_else(|| anyhow!("glTF image view offset overflows"))?;
                if end > parent.0.len() {
                    anyhow::bail!("glTF image view exceeds its buffer");
                }
                parent.0[begin..end].to_vec()
            }
        };
        let decoded =
            astraweave_asset::decode_limited_image(&bytes).context("decode glTF image")?;
        images.push(dynamic_to_gltf_image(decoded));
    }
    Ok(images)
}

#[cfg(feature = "gltf-assets")]
fn load_capped_gltf_buffers(
    document: &gltf::Document,
    base: &std::path::Path,
    mut blob: Option<Vec<u8>>,
) -> Result<Vec<gltf::buffer::Data>> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let declared = buffer.length() as u64;
        if declared > astraweave_asset::MAX_GLTF_BUFFER {
            anyhow::bail!(
                "glTF buffer {} is {declared} bytes, cap is {}",
                buffer.index(),
                astraweave_asset::MAX_GLTF_BUFFER
            );
        }
        let mut data = match buffer.source() {
            gltf::buffer::Source::Bin => blob
                .take()
                .ok_or_else(|| anyhow!("missing glTF BIN for buffer {}", buffer.index()))?,
            gltf::buffer::Source::Uri(uri) if is_data_uri(uri) => {
                astraweave_asset::read_capped_data_uri(uri, astraweave_asset::MAX_GLTF_BUFFER)?
            }
            gltf::buffer::Source::Uri(uri) => {
                let path = jailed_gltf_file(base, uri)?;
                astraweave_asset::read_capped_file(&path, astraweave_asset::MAX_GLTF_BUFFER)?
            }
        };
        if data.len() as u64 > astraweave_asset::MAX_GLTF_BUFFER {
            anyhow::bail!(
                "glTF buffer {} is {} bytes, cap is {}",
                buffer.index(),
                data.len(),
                astraweave_asset::MAX_GLTF_BUFFER
            );
        }
        if (data.len() as u64) < declared {
            anyhow::bail!(
                "glTF buffer {} is shorter than its declared length {declared}",
                buffer.index()
            );
        }
        while data.len() % 4 != 0 {
            data.push(0);
        }
        buffers.push(gltf::buffer::Data(data));
    }
    Ok(buffers)
}

#[cfg(feature = "gltf-assets")]
pub fn load_gltf(path: &std::path::Path, opts: &GltfOptions) -> Result<Vec<CpuMesh>> {
    use gltf::mesh::util::ReadIndices;
    let base = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let gltf::Gltf {
        document: doc,
        blob,
    } = open_capped_gltf(path)?;
    reject_escaping_gltf_uris(&doc, base)?;
    let buffers = load_capped_gltf_buffers(&doc, base, blob)?;
    let images = load_capped_gltf_images(&doc, base, &buffers)?;
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
    attach_sidecar_pbr_maps(path, &mut meshes_out)?;
    Ok(meshes_out)
}

#[cfg(all(test, feature = "gltf-assets"))]
mod jail_tests {
    use super::*;

    /// Embed a path in JSON. Windows `display()` uses `\` which is not a JSON escape.
    fn json_string(value: &str) -> String {
        let mut out = String::from("\"");
        for c in value.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

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

    #[test]
    fn load_gltf_reads_a_jailed_external_buffer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut bin = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend_from_slice(&value.to_le_bytes());
        }
        bin.extend_from_slice(&0u16.to_le_bytes());
        bin.extend_from_slice(&1u16.to_le_bytes());
        bin.extend_from_slice(&2u16.to_le_bytes());
        bin.extend_from_slice(&[0, 0]);
        std::fs::write(dir.path().join("tri.bin"), &bin).expect("write bin");
        let path = dir.path().join("tri.gltf");
        std::fs::write(
            &path,
            r#"{
                "asset": {"version": "2.0"},
                "buffers": [{"uri": "tri.bin", "byteLength": 44}],
                "bufferViews": [
                    {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                    {"buffer": 0, "byteOffset": 36, "byteLength": 6}
                ],
                "accessors": [
                    {
                        "bufferView": 0,
                        "componentType": 5126,
                        "count": 3,
                        "type": "VEC3",
                        "max": [1.0, 1.0, 0.0],
                        "min": [0.0, 0.0, 0.0]
                    },
                    {
                        "bufferView": 1,
                        "componentType": 5123,
                        "count": 3,
                        "type": "SCALAR"
                    }
                ],
                "meshes": [{
                    "primitives": [{
                        "attributes": {"POSITION": 0},
                        "indices": 1
                    }]
                }]
            }"#,
        )
        .expect("write gltf");
        let meshes = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect("load external buffer");
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }

    #[test]
    fn load_gltf_rejects_an_external_buffer_over_256_mib_from_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join("big.bin");
        let file = std::fs::File::create(&bin).expect("create");
        let cap = astraweave_asset::MAX_GLTF_BUFFER;
        file.set_len(cap + 1).expect("set_len");
        drop(file);
        let path = dir.path().join("big.gltf");
        std::fs::write(
            &path,
            r#"{
                "asset": {"version": "2.0"},
                "buffers": [{"uri": "big.bin", "byteLength": 4}]
            }"#,
        )
        .expect("write gltf");
        let err = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("oversized external buffer");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&cap.to_string()) && msg.contains(&(cap + 1).to_string()),
            "{msg}"
        );
    }

    #[test]
    fn load_gltf_rejects_a_declared_buffer_over_256_mib() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("declared.gltf");
        let cap = astraweave_asset::MAX_GLTF_BUFFER;
        let declared = cap + 1;
        std::fs::write(
            &path,
            format!(
                r#"{{
                    "asset": {{"version": "2.0"}},
                    "buffers": [{{
                        "uri": "data:application/octet-stream;base64,AAAA",
                        "byteLength": {declared}
                    }}]
                }}"#
            ),
        )
        .expect("write gltf");
        let err = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("declared buffer over the cap");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&declared.to_string()) && msg.contains(&cap.to_string()),
            "{msg}"
        );
        assert!(!msg.contains("decode"), "{msg}");
    }

    #[test]
    fn load_gltf_rejects_a_glb_header_longer_than_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bomb.glb");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"glTF");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        std::fs::write(&path, &bytes).expect("write glb");
        let started = std::time::Instant::now();
        let err = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("header bomb");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "GLB header allocated before the cap"
        );
        let msg = format!("{err:#}");
        assert!(
            msg.contains("header") && msg.contains(&u32::MAX.to_string()),
            "{msg}"
        );
    }

    #[test]
    fn load_gltf_rejects_a_glb_header_shorter_than_twelve_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("short.glb");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"glTF");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&8u32.to_le_bytes());
        std::fs::write(&path, &bytes).expect("write glb");
        let err = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("short header");
        assert!(
            format!("{err:#}").contains("shorter than the header"),
            "{err}"
        );
    }

    #[test]
    fn load_gltf_rejects_a_file_scheme_and_an_encoded_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("secret.bin");
        std::fs::write(&outside, b"SENTINEL-SECRET").expect("secret");
        let model = dir.path().join("model");
        std::fs::create_dir(&model).expect("model dir");
        let file_uri = json_string(&format!("file://{}", outside.display()));
        let file_gltf = model.join("file.gltf");
        std::fs::write(
            &file_gltf,
            format!(
                r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"uri":{file_uri},"byteLength":15}}]}}"#
            ),
        )
        .expect("write file uri");
        let file_err = load_gltf(
            &file_gltf,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("file scheme");
        let file_msg = format!("{file_err:#}");
        assert!(file_msg.contains("scheme"), "{file_msg}");
        assert!(!file_msg.contains("SENTINEL-SECRET"), "{file_msg}");

        let encoded = model.join("encoded.gltf");
        std::fs::write(
            &encoded,
            r#"{"asset":{"version":"2.0"},"buffers":[{"uri":"%2e%2e/secret.bin","byteLength":15}]}"#,
        )
        .expect("write encoded");
        let encoded_err = load_gltf(
            &encoded,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("encoded parent");
        let encoded_msg = format!("{encoded_err:#}").to_lowercase();
        assert!(
            encoded_msg.contains("traversal") || encoded_msg.contains("rejected"),
            "{encoded_msg}"
        );
    }

    #[test]
    fn load_gltf_opens_a_percent_encoded_buffer_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut bin = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend_from_slice(&value.to_le_bytes());
        }
        bin.extend_from_slice(&0u16.to_le_bytes());
        bin.extend_from_slice(&1u16.to_le_bytes());
        bin.extend_from_slice(&2u16.to_le_bytes());
        bin.extend_from_slice(&[0, 0]);
        std::fs::write(dir.path().join("my albedo.bin"), &bin).expect("write bin");
        let path = dir.path().join("spaced.gltf");
        std::fs::write(
            &path,
            r#"{
                "asset": {"version": "2.0"},
                "buffers": [{"uri": "my%20albedo.bin", "byteLength": 44}],
                "bufferViews": [
                    {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                    {"buffer": 0, "byteOffset": 36, "byteLength": 6}
                ],
                "accessors": [
                    {
                        "bufferView": 0,
                        "componentType": 5126,
                        "count": 3,
                        "type": "VEC3",
                        "max": [1.0, 1.0, 0.0],
                        "min": [0.0, 0.0, 0.0]
                    },
                    {
                        "bufferView": 1,
                        "componentType": 5123,
                        "count": 3,
                        "type": "SCALAR"
                    }
                ],
                "meshes": [{
                    "primitives": [{
                        "attributes": {"POSITION": 0},
                        "indices": 1
                    }]
                }]
            }"#,
        )
        .expect("write gltf");
        let meshes = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect("percent-decoded buffer");
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }
}

#[cfg(feature = "gltf-assets")]
fn image_from_texture<T>(images: &[gltf::image::Data], info: Option<T>) -> Result<Option<CpuImage>>
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

/// Fill empty normal / metallic-roughness / occlusion slots from sibling
/// Poly Haven files.
///
/// Looks beside the model (and in `textures` / `Textures` next to it or its
/// parent) for `*nor_gl*.{png,jpg,jpeg}` and `*_arm*.{png,jpg,jpeg}` whose
/// names contain the model stem or a material hint. `nor_gl` is stored
/// unchanged. ARM is stored as glTF metallic-roughness (G = roughness,
/// B = metallic) plus occlusion, so upload converts it once. Embedded glTF
/// images are left in place. A file that does not share the model stem is
/// ignored, so a pack directory cannot attach a neighbor's maps.
#[cfg(all(feature = "gltf-assets", feature = "textures"))]
pub fn attach_sidecar_pbr_maps(model_path: &std::path::Path, meshes: &mut [CpuMesh]) -> Result<()> {
    use crate::mesh::CpuImage;
    use anyhow::Context;

    if meshes.is_empty() {
        return Ok(());
    }
    let needs_normal = meshes.iter().any(|mesh| mesh.normal_image.is_none());
    let needs_arm = meshes
        .iter()
        .any(|mesh| mesh.metallic_roughness_image.is_none() || mesh.occlusion_image.is_none());
    if !needs_normal && !needs_arm {
        return Ok(());
    }

    let stems = identity_stems(model_path, meshes);
    let mut nor_path = None;
    let mut arm_path = None;
    for dir in sidecar_dirs(model_path) {
        if !dir.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(&dir)
            .with_context(|| format!("read PBR sidecar directory {}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.with_context(|| format!("read PBR sidecar entry in {}", dir.display()))?;
            if !entry
                .file_type()
                .with_context(|| format!("stat {}", entry.path().display()))?
                .is_file()
            {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(kind) = crate::pbr_maps::classify_pbr_sidecar(&name) else {
                continue;
            };
            if !name_matches_stem(&name, &stems) {
                continue;
            }
            match kind {
                crate::pbr_maps::SidecarMapKind::NorGl if nor_path.is_none() => {
                    nor_path = Some(entry.path());
                }
                crate::pbr_maps::SidecarMapKind::Arm if arm_path.is_none() => {
                    arm_path = Some(entry.path());
                }
                _ => {}
            }
        }
    }

    let normal_image = if needs_normal {
        if let Some(path) = nor_path {
            Some(load_rgba8(&path).with_context(|| format!("load nor_gl {}", path.display()))?)
        } else {
            None
        }
    } else {
        None
    };
    let arm_maps = if needs_arm {
        if let Some(path) = arm_path {
            let image =
                load_rgba8(&path).with_context(|| format!("load arm map {}", path.display()))?;
            let (mr, ao) = crate::pbr_maps::arm_to_gltf_mr_and_occlusion(&image.pixels);
            Some((image.width, image.height, mr, ao))
        } else {
            None
        }
    } else {
        None
    };

    for mesh in meshes.iter_mut() {
        if mesh.normal_image.is_none() {
            if let Some(image) = &normal_image {
                mesh.normal_image = Some(image.clone());
            }
        }
        if let Some((width, height, mr, ao)) = &arm_maps {
            if mesh.metallic_roughness_image.is_none() {
                mesh.metallic_roughness_image = Some(CpuImage {
                    width: *width,
                    height: *height,
                    pixels: mr.clone(),
                });
            }
            if mesh.occlusion_image.is_none() {
                mesh.occlusion_image = Some(CpuImage {
                    width: *width,
                    height: *height,
                    pixels: ao.clone(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(not(all(feature = "gltf-assets", feature = "textures")))]
pub fn attach_sidecar_pbr_maps(
    _model_path: &std::path::Path,
    _meshes: &mut [CpuMesh],
) -> Result<()> {
    Ok(())
}

#[cfg(all(feature = "gltf-assets", feature = "textures"))]
fn identity_stems(model_path: &std::path::Path, meshes: &[CpuMesh]) -> Vec<String> {
    let mut stems = Vec::new();
    if let Some(stem) = model_path.file_stem().and_then(|stem| stem.to_str()) {
        push_stem(&mut stems, stem);
    }
    for mesh in meshes {
        if let Some(hint) = &mesh.texture_source_hint {
            push_stem(&mut stems, hint);
            let lower = hint.to_ascii_lowercase();
            for suffix in [
                "_diff",
                "_albedo",
                "_basecolor",
                "_base_color",
                "_color",
                "_diffuse",
            ] {
                if let Some(stripped) = lower.strip_suffix(suffix) {
                    push_stem(&mut stems, stripped);
                }
            }
        }
    }
    stems
}

#[cfg(all(feature = "gltf-assets", feature = "textures"))]
fn push_stem(stems: &mut Vec<String>, stem: &str) {
    let lower = stem.to_ascii_lowercase();
    if lower.len() >= 3 && !stems.iter().any(|existing| existing == &lower) {
        stems.push(lower);
    }
}

#[cfg(all(feature = "gltf-assets", feature = "textures"))]
fn name_matches_stem(file_name: &str, stems: &[String]) -> bool {
    let lower = file_name.to_ascii_lowercase();
    stems.iter().any(|stem| lower.contains(stem))
}

#[cfg(all(feature = "gltf-assets", feature = "textures"))]
fn sidecar_dirs(model_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(parent) = model_path.parent() {
        dirs.push(parent.to_path_buf());
        dirs.push(parent.join("textures"));
        dirs.push(parent.join("Textures"));
        if let Some(grand) = parent.parent() {
            dirs.push(grand.join("textures"));
            dirs.push(grand.join("Textures"));
        }
    }
    dirs
}

#[cfg(all(feature = "gltf-assets", feature = "textures"))]
fn load_rgba8(path: &std::path::Path) -> Result<crate::mesh::CpuImage> {
    let bytes = astraweave_asset::read_limited_image_bytes(path)?;
    let image = astraweave_asset::decode_limited_image(&bytes)?.to_rgba8();
    Ok(crate::mesh::CpuImage {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
    })
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

#[cfg(all(test, feature = "gltf-assets", feature = "textures"))]
mod tests {
    use super::*;
    use crate::mesh::{CpuImage, CpuMesh};

    fn write_png(path: &std::path::Path, pixel: [u8; 4]) {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba(pixel));
        image.save(path).expect("write sidecar png");
    }

    #[test]
    fn polyhaven_sidecars_fill_empty_slots_and_leave_embedded_maps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = dir.path().join("barrel.glb");
        std::fs::write(&model, b"not-a-glb").expect("write stub glb");
        write_png(
            &dir.path().join("barrel_nor_gl_2k.png"),
            [128, 200, 255, 255],
        );
        write_png(&dir.path().join("barrel_arm_2k.png"), [10, 80, 250, 255]);
        write_png(&dir.path().join("neighbor_nor_gl_2k.png"), [1, 2, 3, 255]);

        let embedded_normal = CpuImage {
            width: 1,
            height: 1,
            pixels: vec![9, 9, 9, 255],
        };
        let mut meshes = vec![
            CpuMesh::default(),
            CpuMesh {
                normal_image: Some(embedded_normal.clone()),
                ..CpuMesh::default()
            },
        ];
        attach_sidecar_pbr_maps(&model, &mut meshes).expect("attach");

        let normal = meshes[0].normal_image.as_ref().expect("nor_gl");
        assert_eq!(&normal.pixels[..3], &[128, 200, 255]);
        let mr = meshes[0]
            .metallic_roughness_image
            .as_ref()
            .expect("arm as gltf mr");
        assert_eq!(mr.pixels[1], 80);
        assert_eq!(mr.pixels[2], 250);
        let ao = meshes[0].occlusion_image.as_ref().expect("arm ao");
        assert_eq!(ao.pixels[0], 10);

        let kept = meshes[1].normal_image.as_ref().expect("embedded normal");
        assert_eq!(kept.pixels, embedded_normal.pixels);
        assert!(meshes[1].metallic_roughness_image.is_some());
    }

    #[test]
    fn unrelated_sidecar_is_not_attached() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = dir.path().join("rogue.glb");
        std::fs::write(&model, b"not-a-glb").expect("write stub glb");
        write_png(&dir.path().join("warm_armor.png"), [1, 2, 3, 255]);
        write_png(&dir.path().join("crate_nor_gl.png"), [4, 5, 6, 255]);
        let mut meshes = vec![CpuMesh::default()];
        attach_sidecar_pbr_maps(&model, &mut meshes).expect("attach");
        assert!(meshes[0].normal_image.is_none());
        assert!(meshes[0].metallic_roughness_image.is_none());
        assert!(meshes[0].occlusion_image.is_none());
    }

    #[test]
    fn load_gltf_rejects_an_oversized_data_image_before_decode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("huge.gltf");
        let payload_len = (astraweave_asset::image_limits::MAX_IMAGE_ALLOC as usize * 4 / 3) + 8;
        let mut doc = String::from(
            r#"{"asset":{"version":"2.0"},"images":[{"uri":"data:application/octet-stream;base64,"#,
        );
        doc.extend(std::iter::repeat('A').take(payload_len));
        doc.push_str(r#""}]}"#);
        std::fs::write(&path, doc).expect("write gltf");
        let err = load_gltf(
            &path,
            &GltfOptions {
                generate_tangents: false,
            },
        )
        .expect_err("oversized data image");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&astraweave_asset::image_limits::MAX_IMAGE_ALLOC.to_string()),
            "{msg}"
        );
    }
}
