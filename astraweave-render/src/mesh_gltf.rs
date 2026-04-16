#[cfg(feature = "gltf-assets")]
use crate::mesh::{compute_tangents, MeshVertex};
use crate::mesh::{CpuImage, CpuMesh};
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
pub fn load_gltf(path: &std::path::Path, opts: &GltfOptions) -> Result<Vec<CpuMesh>> {
    use gltf::mesh::util::ReadIndices;
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
                albedo_image: None,
                texture_source_hint: None,
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
            // Extract base color texture from glTF material
            cpu.albedo_image = prim
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
                .and_then(|info| {
                    let idx = info.texture().source().index();
                    images.get(idx).map(|img| {
                        let pixels = convert_gltf_image_to_rgba8(img);
                        CpuImage {
                            width: img.width,
                            height: img.height,
                            pixels,
                        }
                    })
                });

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

#[cfg(feature = "gltf-assets")]
fn convert_gltf_image_to_rgba8(img: &gltf::image::Data) -> Vec<u8> {
    match img.format {
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
        _ => {
            // R16/R16G16/R16G16B16/R16G16B16A16/R32G32B32FLOAT/R32G32B32A32FLOAT
            // Fallback: 1x1 white
            vec![255, 255, 255, 255]
        }
    }
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
