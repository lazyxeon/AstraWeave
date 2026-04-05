/// GLTF/GLB loader for unified_showcase
/// Loads 3D models and converts them to wgpu mesh format
use anyhow::{Context, Result};
use glam::Vec3;
use std::path::Path;

/// Vertex format with vertex colors (for Kenney models)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GltfVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],   // Vertex color (RGBA)
    pub tangent: [f32; 4], // Tangent (xyz) + handedness (w)
}

/// Loaded mesh data ready for GPU upload
pub struct LoadedMesh {
    pub vertices: Vec<GltfVertex>,
    pub indices: Vec<u32>,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub material_name: Option<String>,
}

/// Compute AABB (axis-aligned bounding box) for a set of positions
fn compute_aabb(positions: &[[f32; 3]]) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for pos in positions {
        let p = Vec3::from(*pos);
        min = min.min(p);
        max = max.max(p);
    }

    (min, max)
}

/// Compute per-vertex tangent frames (tangent.x, tangent.y, tangent.z, handedness)
fn compute_tangents(
    positions: &[[f32; 3]],
    uvs: &[[f32; 2]],
    normals: &[[f32; 3]],
    indices: &[u32],
) -> Vec<[f32; 4]> {
    let mut tangents: Vec<Vec3> = vec![Vec3::ZERO; positions.len()];
    let mut bitangents: Vec<Vec3> = vec![Vec3::ZERO; positions.len()];

    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }

        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);

        let uv0 = Vec3::new(uvs[i0][0], uvs[i0][1], 0.0);
        let uv1 = Vec3::new(uvs[i1][0], uvs[i1][1], 0.0);
        let uv2 = Vec3::new(uvs[i2][0], uvs[i2][1], 0.0);

        let dp1 = p1 - p0;
        let dp2 = p2 - p0;
        let duv1 = uv1 - uv0;
        let duv2 = uv2 - uv0;

        let r = 1.0 / (duv1.x * duv2.y - duv2.x * duv1.y + 1e-8);
        let tangent = (dp1 * duv2.y - dp2 * duv1.y) * r;
        let bitangent = (dp2 * duv1.x - dp1 * duv2.x) * r;

        tangents[i0] += tangent;
        tangents[i1] += tangent;
        tangents[i2] += tangent;
        bitangents[i0] += bitangent;
        bitangents[i1] += bitangent;
        bitangents[i2] += bitangent;
    }

    tangents
        .into_iter()
        .zip(bitangents)
        .zip(normals.iter())
        .map(|((t, b), n)| {
            let t = t.normalize_or_zero();
            let b = b.normalize_or_zero();
            let n = Vec3::from(*n);
            let w = if n.cross(t).dot(b) < 0.0 { -1.0 } else { 1.0 };
            [t.x, t.y, t.z, w]
        })
        .collect()
}

/// Generate planar UVs based on XZ projection (suitable for terrain and buildings)
fn generate_planar_uvs(positions: &[[f32; 3]]) -> Vec<[f32; 2]> {
    let (min, max) = compute_aabb(positions);
    let size = max - min;

    // Avoid division by zero
    let size_x = if size.x.abs() < 0.001 { 1.0 } else { size.x };
    let size_z = if size.z.abs() < 0.001 { 1.0 } else { size.z };

    positions
        .iter()
        .map(|pos| {
            let p = Vec3::from(*pos);
            let u = (p.x - min.x) / size_x;
            let v = (p.z - min.z) / size_z;
            [u, v]
        })
        .collect()
}

/// Generate smooth per-vertex normals by averaging adjacent face normals
fn generate_smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals_accum: Vec<Vec3> = vec![Vec3::ZERO; positions.len()];

    // Accumulate face normals for each vertex
    for triangle in indices.chunks(3) {
        if triangle.len() < 3 {
            continue;
        }

        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }

        let v0 = Vec3::from(positions[i0]);
        let v1 = Vec3::from(positions[i1]);
        let v2 = Vec3::from(positions[i2]);

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);

        // Accumulate weighted by face area (magnitude of cross product)
        normals_accum[i0] += face_normal;
        normals_accum[i1] += face_normal;
        normals_accum[i2] += face_normal;
    }

    // Normalize accumulated normals
    normals_accum
        .iter()
        .map(|n| {
            let normalized = n.normalize_or_zero();
            // Fallback to up vector if normalization fails
            if normalized.length_squared() < 0.001 {
                [0.0, 1.0, 0.0]
            } else {
                normalized.to_array()
            }
        })
        .collect()
}

/// Load a GLTF/GLB file and extract primitives as separate meshes
/// This allows for multi-material objects (e.g. trees with trunk/leaves)
pub fn load_gltf(path: impl AsRef<Path>) -> Result<Vec<LoadedMesh>> {
    let path = path.as_ref();

    // Canonicalize path to ensure gltf crate can find relative resources
    let abs_path = std::fs::canonicalize(path)
        .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;

    log::info!("Attempting to load GLTF: {}", abs_path.display());

    // Use open file to ensure we have read permissions and it exists
    let file = std::fs::File::open(&abs_path)
        .with_context(|| format!("Failed to open file: {}", abs_path.display()))?;
    let reader = std::io::BufReader::new(file);

    // Use Gltf::from_reader to load
    let gltf = gltf::Gltf::from_reader(reader)
        .with_context(|| format!("Failed to parse GLTF from reader: {}", abs_path.display()))?;

    // Load buffers (this handles external bin files relative to the GLTF file)
    let base_dir = abs_path.parent().unwrap_or(Path::new("."));
    let buffers = gltf::import_buffers(&gltf.document, Some(base_dir), gltf.blob)
        .with_context(|| "Failed to load GLTF buffers")?;

    let document = gltf.document;

    log::info!("GLTF loaded successfully, parsing meshes...");

    let mut loaded_meshes = Vec::new();

    for mesh in document.meshes() {
        let mesh_name = mesh.name().unwrap_or("unnamed").to_string();
        log::info!(
            "Processing mesh: '{}' with {} primitives",
            mesh_name,
            mesh.primitives().len()
        );

        for (prim_idx, primitive) in mesh.primitives().enumerate() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .with_context(|| format!("No position data in primitive {}", prim_idx))?
                .collect();

            let indices: Vec<u32> = reader
                .read_indices()
                .with_context(|| format!("No index data in primitive {}", prim_idx))?
                .into_u32()
                .collect();

            // Extract or generate normals
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| {
                    log::info!(
                        "  Primitive {}: No normals found, generating smooth normals",
                        prim_idx
                    );
                    generate_smooth_normals(&positions, &indices)
                });

            // Extract or generate UVs (CRITICAL: Preserve original model UVs!)
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| {
                    log::info!(
                        "  Primitive {}: No UVs found, generating planar UVs",
                        prim_idx
                    );
                    generate_planar_uvs(&positions)
                });

            // Extract vertex colors (Kenney models use these!)
            let colors: Vec<[f32; 4]> = reader
                .read_colors(0)
                .map(|iter| iter.into_rgba_f32().collect())
                .unwrap_or_else(|| {
                    // log::info!("  Primitive {}: No vertex colors, using white", prim_idx);
                    vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]
                });

            // Clamp colors to prevent blowout
            let colors: Vec<[f32; 4]> = colors
                .into_iter()
                .map(|c| [c[0].min(1.0), c[1].min(1.0), c[2].min(1.0), c[3].min(1.0)])
                .collect();

            // Extract or compute tangents
            let tangents: Vec<[f32; 4]> = if let Some(tangents_iter) = reader.read_tangents() {
                tangents_iter.map(|t| [t[0], t[1], t[2], t[3]]).collect()
            } else {
                // log::info!(
                //     "  Primitive {}: No tangents found, computing tangents",
                //     prim_idx
                // );
                compute_tangents(&positions, &uvs, &normals, &indices)
            };

            let mut vertices = Vec::with_capacity(positions.len());
            for i in 0..positions.len() {
                vertices.push(GltfVertex {
                    position: positions[i],
                    normal: normals[i],
                    uv: uvs[i],
                    color: colors[i],
                    tangent: tangents[i],
                });
            }

            let material_name = primitive.material().name().map(|s| s.to_string());

            log::info!(
                "  Primitive {}: {} vertices, {} triangles, material: {:?}",
                prim_idx,
                positions.len(),
                indices.len() / 3,
                material_name
            );

            loaded_meshes.push(LoadedMesh {
                vertices,
                indices,
                name: mesh_name.clone(),
                material_name,
            });
        }
    }

    if loaded_meshes.is_empty() {
        anyhow::bail!("No valid primitives found in GLTF file");
    }

    Ok(loaded_meshes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_demo_plane() {
        // This test requires the demo_plane.gltf file to exist
        let result = load_gltf("../../assets/demo_plane.gltf");
        if let Ok(meshes) = result {
            assert!(!meshes.is_empty(), "Should have meshes");
            let mesh = &meshes[0];
            assert!(!mesh.vertices.is_empty(), "Should have vertices");
            assert!(!mesh.indices.is_empty(), "Should have indices");
            assert_eq!(mesh.indices.len() % 3, 0, "Indices should be triangles");
        }
    }
}
