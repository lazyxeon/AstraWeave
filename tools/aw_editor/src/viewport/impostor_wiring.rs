//! Impostor wiring helpers (Phase 5.3 T7 stage 3c.1 — editor).
//!
//! This module bridges the editor's scatter LOD3 pipeline to the shared
//! [`astraweave_render::impostor_bake`] / [`astraweave_render::impostor_pass`]
//! infrastructure. It provides two narrow primitives:
//!
//! 1. [`primitive_mesh_hash`] — deterministic content hash of a
//!    [`astraweave_render::mesh::CpuMesh`] (vertex + index + albedo bytes),
//!    used as the key for [`super::impostor_registry::ImpostorRegistry`] and
//!    for [`astraweave_render::Renderer::install_impostor_pass`] under the
//!    HashMap API landed in stage 3b.
//!
//! 2. [`bake_primitive_pixels`] — turns a CpuMesh + [`ImpostorAtlasSpec`]
//!    into baked RGBA8 atlas pixels by spinning up an [`ImpostorBaker`] and
//!    driving it through every species × angle cell.
//!
//! Stage scope: foundations only. The LOD3 pipeline in `engine_adapter.rs`
//! continues to route through the legacy PBR-quad path; stage 3c.2 will
//! swap that for `Renderer::install_impostor_pass` + `ImpostorPass::record`.
//!
//! Everything in this module is feature-gated behind `impostor-bake` at the
//! module-include site (`viewport/mod.rs`), since the bake pipeline pulls in
//! additional render-crate code paths we only want in editor builds that
//! have opted in.

use anyhow::{anyhow, Context, Result};
use astraweave_render::impostor_bake::{
    fit_ortho_camera, upload_simplification_mesh, Aabb, ImpostorBaker, ImpostorBakerConfig,
};
use astraweave_render::lod_generator::SimplificationMesh;
use astraweave_render::mesh::{CpuImage, CpuMesh};
use astraweave_render::vegetation_lod::ImpostorAtlasSpec;
use glam::Vec3;
use sha2::{Digest, Sha256};

use super::impostor_registry::MeshHash;

/// Derive a stable content hash for a single scatter primitive.
///
/// The hash covers vertex positions / normals / UVs, the index buffer, and
/// the albedo bytes (if any). It is deliberately insensitive to transient
/// state (instance transforms, draw-call names) so that two distinct scatter
/// groups sharing the same mesh and texture will collide on the same cached
/// atlas.
pub fn primitive_mesh_hash(mesh: &CpuMesh) -> MeshHash {
    let mut hasher = Sha256::new();
    // Vertex data. `MeshVertex` is `#[repr(C)]` (see astraweave-render/src/mesh.rs)
    // so casting the slice to raw bytes is deterministic across runs.
    hasher.update((mesh.vertices.len() as u64).to_le_bytes());
    hasher.update(bytemuck::cast_slice(&mesh.vertices));
    // Index buffer.
    hasher.update((mesh.indices.len() as u64).to_le_bytes());
    hasher.update(bytemuck::cast_slice(&mesh.indices));
    // Albedo bytes (if present). We include w/h so two images of identical
    // pixel length but different dimensions don't collide.
    if let Some(img) = mesh.albedo_image.as_ref() {
        hasher.update(b"albedo:");
        hasher.update(img.width.to_le_bytes());
        hasher.update(img.height.to_le_bytes());
        hasher.update(&img.pixels);
    } else {
        hasher.update(b"albedo:none");
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest.iter() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{:02x}", b);
    }
    MeshHash::from_hex(&hex).expect("SHA-256 digest is always 64 hex chars")
}

/// Convert a scatter [`CpuMesh`] into the [`SimplificationMesh`] layout the
/// [`ImpostorBaker`] expects. Tangent data is dropped — the bake shader is
/// unlit (see `impostor_bake.rs`'s `IMPOSTOR_BAKE_WGSL`), so only positions
/// and UVs actually drive the output.
fn to_simplification_mesh(mesh: &CpuMesh) -> SimplificationMesh {
    let positions: Vec<Vec3> = mesh
        .vertices
        .iter()
        .map(|v| Vec3::from_array(v.position))
        .collect();
    let normals: Vec<Vec3> = mesh
        .vertices
        .iter()
        .map(|v| Vec3::from_array(v.normal))
        .collect();
    let uvs: Vec<[f32; 2]> = mesh.vertices.iter().map(|v| v.uv).collect();
    SimplificationMesh::new(positions, normals, uvs, mesh.indices.clone())
}

/// Upload the primitive's albedo texture to the GPU for bake sampling. If the
/// mesh has no `albedo_image`, a 1×1 opaque white texture is synthesized so
/// the unlit bake shader still produces sensible pixels (UV-mapped geometry
/// renders its vertex color defaults, which matches the legacy PBR-quad
/// fallback's visual target).
fn upload_bake_diffuse(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    baker: &ImpostorBaker,
    albedo: Option<&CpuImage>,
    label: &str,
) -> wgpu::BindGroup {
    let (width, height, pixels) = match albedo {
        Some(img) if img.width > 0 && img.height > 0 => {
            (img.width, img.height, img.pixels.clone())
        }
        _ => (1u32, 1u32, vec![255u8, 255, 255, 255]),
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("impostor-bake-diffuse-{label}")),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    baker.make_diffuse_bind_group(device, &view)
}

/// Bake a single-species atlas for the given [`CpuMesh`].
///
/// Returns RGBA8 pixel data sized `spec.atlas_width × spec.atlas_height × 4`.
/// The returned pixels are suitable for feeding directly to
/// [`astraweave_render::impostor_pass::ImpostorPass::new`] or for persisting
/// via [`astraweave_render::impostor_bake::load_or_bake_atlas`]'s bake
/// closure.
///
/// # Preconditions
///
/// * `spec.species.len() == 1` — this bakes a *single* primitive. Multi-
///   species atlases are owned by the CLI `aw-impostor-bake` binary, not the
///   editor runtime path (one atlas per scatter primitive is simpler and
///   matches the registry's per-hash keying).
/// * `mesh.vertices` must be non-empty (otherwise AABB fitting fails).
///
/// # Errors
///
/// Returns an error if the baker rejects the atlas dimensions, the mesh has
/// no vertex positions, the spec is missing a cell, or readback fails.
pub fn bake_primitive_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mesh: &CpuMesh,
    spec: &ImpostorAtlasSpec,
    label: &str,
) -> Result<(Vec<u8>, u32, u32)> {
    if spec.species.len() != 1 {
        anyhow::bail!(
            "bake_primitive_pixels: expected exactly 1 species in spec, got {}",
            spec.species.len()
        );
    }
    if mesh.vertices.is_empty() {
        anyhow::bail!("bake_primitive_pixels: mesh '{}' has no vertices", label);
    }

    let atlas_w = spec.atlas_width;
    let atlas_h = spec.atlas_height;
    let angle_count = spec.angle_count;

    let baker = ImpostorBaker::new(
        device,
        ImpostorBakerConfig {
            atlas_width: atlas_w,
            atlas_height: atlas_h,
            sample_count: 1,
        },
    )
    .with_context(|| format!("creating ImpostorBaker for '{label}'"))?;
    baker.clear(device, queue);

    let simp = to_simplification_mesh(mesh);
    let (vbuf, ibuf, icount) = upload_simplification_mesh(device, &simp);
    let diffuse_bg = upload_bake_diffuse(device, queue, &baker, mesh.albedo_image.as_ref(), label);

    let aabb = Aabb::from_points(&simp.positions)
        .ok_or_else(|| anyhow!("bake_primitive_pixels: mesh '{}' AABB is empty", label))?;

    for angle_idx in 0..angle_count {
        let angle = angle_idx as f32 * std::f32::consts::TAU / angle_count as f32;
        let (proj, view) = fit_ortho_camera(aabb, angle);
        let region = spec.lookup(0, angle).copied().ok_or_else(|| {
            anyhow!(
                "bake_primitive_pixels: spec missing cell for species 0 angle {}",
                angle_idx
            )
        })?;
        baker.draw_into_region(
            device, queue, proj * view, region, &vbuf, &ibuf, icount, &diffuse_bg,
        );
    }

    let pixels = baker
        .readback_atlas(device, queue)
        .with_context(|| format!("readback_atlas for '{label}'"))?;
    Ok((pixels, atlas_w, atlas_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_render::mesh::MeshVertex;

    fn fake_mesh() -> CpuMesh {
        CpuMesh {
            vertices: vec![
                MeshVertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                },
                MeshVertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [1.0, 0.0],
                },
                MeshVertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            albedo_image: None,
            texture_source_hint: None,
        }
    }

    #[test]
    fn hash_is_deterministic_and_content_addressed() {
        let a = fake_mesh();
        let b = fake_mesh();
        assert_eq!(primitive_mesh_hash(&a), primitive_mesh_hash(&b));

        let mut c = fake_mesh();
        c.vertices[0].position[0] = 0.5;
        assert_ne!(primitive_mesh_hash(&a), primitive_mesh_hash(&c));
    }

    #[test]
    fn hash_changes_when_albedo_differs() {
        let mut a = fake_mesh();
        let mut b = fake_mesh();
        a.albedo_image = Some(CpuImage {
            width: 2,
            height: 2,
            pixels: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255],
        });
        b.albedo_image = Some(CpuImage {
            width: 2,
            height: 2,
            pixels: vec![0, 255, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255],
        });
        assert_ne!(primitive_mesh_hash(&a), primitive_mesh_hash(&b));
    }

    #[test]
    fn hash_differs_for_none_vs_some_albedo() {
        let a = fake_mesh();
        let mut b = fake_mesh();
        b.albedo_image = Some(CpuImage {
            width: 1,
            height: 1,
            pixels: vec![128, 128, 128, 255],
        });
        assert_ne!(primitive_mesh_hash(&a), primitive_mesh_hash(&b));
    }

    #[test]
    fn to_simplification_mesh_preserves_topology() {
        let m = fake_mesh();
        let s = to_simplification_mesh(&m);
        assert_eq!(s.positions.len(), 3);
        assert_eq!(s.normals.len(), 3);
        assert_eq!(s.uvs.len(), 3);
        assert_eq!(s.indices, vec![0, 1, 2]);
        assert_eq!(s.positions[1].x, 1.0);
    }
}
