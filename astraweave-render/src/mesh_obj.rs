#[allow(unused_imports)]
use anyhow::{anyhow, Result};

use crate::mesh::CpuMesh;
#[cfg(feature = "obj-assets")]
use crate::mesh::MeshVertex;
#[cfg(feature = "obj-assets")]
use glam::{Vec2, Vec3, Vec4};

#[cfg(feature = "obj-assets")]
pub fn load_obj(path: &std::path::Path) -> Result<Vec<CpuMesh>> {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )?;
    let mut out = Vec::new();
    for m in models {
        let mesh = m.mesh;
        if mesh.positions.is_empty() {
            continue;
        }
        let mut cpu = CpuMesh {
            vertices: Vec::with_capacity(mesh.positions.len() / 3),
            indices: mesh.indices.clone(),
            albedo_image: None,
            texture_source_hint: None,
        };
        for i in 0..(mesh.positions.len() / 3) {
            let p = Vec3::new(
                mesh.positions[3 * i],
                mesh.positions[3 * i + 1],
                mesh.positions[3 * i + 2],
            );
            let n = if !mesh.normals.is_empty() {
                Vec3::new(
                    mesh.normals[3 * i],
                    mesh.normals[3 * i + 1],
                    mesh.normals[3 * i + 2],
                )
            } else {
                Vec3::Y
            };
            let uv = if !mesh.texcoords.is_empty() {
                Vec2::new(mesh.texcoords[2 * i], mesh.texcoords[2 * i + 1])
            } else {
                Vec2::ZERO
            };
            cpu.vertices
                .push(MeshVertex::new(p, n, Vec4::new(1.0, 0.0, 0.0, 1.0), uv));
        }
        // Basic tangent generation similar to glTF path
        super::mesh_gltf::generate_mikktspace_tangents(&mut cpu)?;
        out.push(cpu);
    }
    Ok(out)
}

#[cfg(not(feature = "obj-assets"))]
pub fn load_obj(_path: &std::path::Path) -> Result<Vec<CpuMesh>> {
    Err(anyhow!("obj-assets feature not enabled"))
}
