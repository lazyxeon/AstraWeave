use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec4};

// Canonical vertex layout for engine meshes: P/N/T/UV
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4], // xyz=tangent, w=handedness
    pub uv: [f32; 2],
}

impl MeshVertex {
    pub fn new(position: Vec3, normal: Vec3, tangent: Vec4, uv: Vec2) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            tangent: tangent.to_array(),
            uv: uv.to_array(),
        }
    }
    pub fn from_arrays(
        position: [f32; 3],
        normal: [f32; 3],
        tangent: [f32; 4],
        uv: [f32; 2],
    ) -> Self {
        Self {
            position,
            normal,
            tangent,
            uv,
        }
    }
    pub const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x3, // position
        1 => Float32x3, // normal
        2 => Float32x4, // tangent
        3 => Float32x2, // uv
    ];
}

pub struct MeshVertexLayout;
impl MeshVertexLayout {
    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &MeshVertex::ATTRIBS,
        }
    }
}

/// Raw RGBA8 image data extracted from a glTF material.
#[derive(Clone, Debug)]
pub struct CpuImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8
}

#[derive(Clone, Debug, Default)]
pub struct CpuMesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    /// Base color texture from the glTF material (if present).
    pub albedo_image: Option<CpuImage>,
    /// Hint for external texture discovery: the filename stem of the
    /// glTF image source URI (e.g. `"island_tree_01_leaves_diff"`).
    /// Populated even when the image data itself failed to load, so
    /// that the scatter uploader can search for the file on disk.
    pub texture_source_hint: Option<String>,
}

impl CpuMesh {
    pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
        if self.vertices.is_empty() {
            return None;
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in &self.vertices {
            let p = Vec3::from_array(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        Some((min, max))
    }
}

// MikkTSpace tangent generation (spec-compliant)
//
// Uses the `mikktspace` crate which implements the exact algorithm from
// Morten Mikkelsen's paper. This guarantees tangent-space consistency
// with normal maps authored in MikkTSpace-aware tools (Blender, Substance, xNormal).

/// Adapter for `mikktspace::Geometry` over indexed triangle meshes.
struct MikkTSpaceAdapter<'a> {
    vertices: &'a mut [MeshVertex],
    indices: &'a [u32],
}

impl mikktspace::Geometry for MikkTSpaceAdapter<'_> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        let idx = self.indices[face * 3 + vert] as usize;
        self.vertices[idx].position
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        let idx = self.indices[face * 3 + vert] as usize;
        self.vertices[idx].normal
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        let idx = self.indices[face * 3 + vert] as usize;
        self.vertices[idx].uv
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        let idx = self.indices[face * 3 + vert] as usize;
        self.vertices[idx].tangent = tangent;
    }
}

/// Compute MikkTSpace-compliant tangents for an indexed triangle mesh.
///
/// Falls back to a Lengyel approximation if the mesh has degenerate
/// geometry that MikkTSpace cannot handle (e.g., zero-area triangles).
pub fn compute_tangents(mesh: &mut CpuMesh) {
    if mesh.indices.len() % 3 != 0 || mesh.indices.is_empty() {
        return;
    }

    // MikkTSpace panics on degenerate triangles where all three indices
    // reference the same vertex (causes out-of-bounds in the algorithm).
    // Check for this and fall back to Lengyel if found.
    let has_degenerate = mesh
        .indices
        .chunks_exact(3)
        .any(|tri| tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2]);
    if has_degenerate {
        compute_tangents_lengyel(mesh);
        return;
    }

    let mut adapter = MikkTSpaceAdapter {
        vertices: &mut mesh.vertices,
        indices: &mesh.indices,
    };

    if !mikktspace::generate_tangents(&mut adapter) {
        // MikkTSpace can fail on degenerate geometry — fall back to
        // Lengyel approximation so we never leave tangents unset.
        compute_tangents_lengyel(mesh);
    }
}

/// Lengyel approximation fallback (Gram-Schmidt orthonormalization + handedness).
fn compute_tangents_lengyel(mesh: &mut CpuMesh) {
    let v = &mut mesh.vertices;
    let idx = &mesh.indices;
    let mut tan1: Vec<Vec3> = vec![Vec3::ZERO; v.len()];
    let mut tan2: Vec<Vec3> = vec![Vec3::ZERO; v.len()];
    for tri in idx.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = Vec3::from_array(v[i0].position);
        let p1 = Vec3::from_array(v[i1].position);
        let p2 = Vec3::from_array(v[i2].position);
        let uv0 = Vec2::from_array(v[i0].uv);
        let uv1 = Vec2::from_array(v[i1].uv);
        let uv2 = Vec2::from_array(v[i2].uv);
        let dp1 = p1 - p0;
        let dp2 = p2 - p0;
        let duv1 = uv1 - uv0;
        let duv2 = uv2 - uv0;
        let r = 1.0 / (duv1.x * duv2.y - duv1.y * duv2.x).max(1e-8);
        let sdir = (dp1 * duv2.y - dp2 * duv1.y) * r;
        let tdir = (dp2 * duv1.x - dp1 * duv2.x) * r;
        tan1[i0] += sdir;
        tan1[i1] += sdir;
        tan1[i2] += sdir;
        tan2[i0] += tdir;
        tan2[i1] += tdir;
        tan2[i2] += tdir;
    }
    for i in 0..v.len() {
        let n = Vec3::from_array(v[i].normal).normalize_or_zero();
        let t = tan1[i];
        let tangent = (t - n * n.dot(t)).normalize_or_zero();
        let w = if n.cross(t).dot(tan2[i]) < 0.0 {
            -1.0
        } else {
            1.0
        };
        v[i].tangent = [tangent.x, tangent.y, tangent.z, w];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_vertex_new() {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let norm = Vec3::new(0.0, 1.0, 0.0);
        let tan = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let uv = Vec2::new(0.5, 0.75);

        let vert = MeshVertex::new(pos, norm, tan, uv);

        assert_eq!(vert.position, [1.0, 2.0, 3.0]);
        assert_eq!(vert.normal, [0.0, 1.0, 0.0]);
        assert_eq!(vert.tangent, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vert.uv, [0.5, 0.75]);
    }

    #[test]
    fn test_mesh_vertex_from_arrays() {
        let vert = MeshVertex::from_arrays(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.5, 0.75],
        );

        assert_eq!(vert.position, [1.0, 2.0, 3.0]);
        assert_eq!(vert.normal, [0.0, 1.0, 0.0]);
        assert_eq!(vert.tangent, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vert.uv, [0.5, 0.75]);
    }

    #[test]
    fn test_mesh_vertex_layout() {
        let layout = MeshVertexLayout::buffer_layout();

        // Verify stride: 3f32 + 3f32 + 4f32 + 2f32 = 12 floats = 48 bytes
        assert_eq!(layout.array_stride, 48);
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);

        // Verify attributes
        assert_eq!(layout.attributes.len(), 4);
    }

    #[test]
    fn test_mesh_vertex_attribs_locations() {
        // Verify shader locations are correct
        assert_eq!(MeshVertex::ATTRIBS[0].shader_location, 0); // position
        assert_eq!(MeshVertex::ATTRIBS[1].shader_location, 1); // normal
        assert_eq!(MeshVertex::ATTRIBS[2].shader_location, 2); // tangent
        assert_eq!(MeshVertex::ATTRIBS[3].shader_location, 3); // uv
    }

    #[test]
    fn test_cpu_mesh_default() {
        let mesh = CpuMesh::default();
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn test_cpu_mesh_aabb_empty() {
        let mesh = CpuMesh::default();
        assert_eq!(mesh.aabb(), None, "Empty mesh should have no AABB");
    }

    #[test]
    fn test_cpu_mesh_aabb_single_vertex() {
        let mut mesh = CpuMesh::default();
        mesh.vertices.push(MeshVertex::from_arrays(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));

        let (min, max) = mesh.aabb().expect("Should have AABB");
        assert_eq!(min, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(max, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_cpu_mesh_aabb_multiple_vertices() {
        let mut mesh = CpuMesh::default();
        mesh.vertices.push(MeshVertex::from_arrays(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
        mesh.vertices.push(MeshVertex::from_arrays(
            [5.0, 1.0, 7.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
        mesh.vertices.push(MeshVertex::from_arrays(
            [-2.0, 4.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));

        let (min, max) = mesh.aabb().expect("Should have AABB");
        assert_eq!(min, Vec3::new(-2.0, 1.0, 0.0));
        assert_eq!(max, Vec3::new(5.0, 4.0, 7.0));
    }

    #[test]
    fn test_compute_tangents_empty() {
        let mut mesh = CpuMesh::default();
        compute_tangents(&mut mesh); // Should not crash
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn test_compute_tangents_incomplete_triangle() {
        let mut mesh = CpuMesh::default();
        mesh.vertices.push(MeshVertex::from_arrays(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
        mesh.vertices.push(MeshVertex::from_arrays(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0],
        ));
        mesh.indices = vec![0, 1]; // Not divisible by 3

        compute_tangents(&mut mesh); // Should not crash, just return early
        assert_eq!(mesh.indices.len(), 2);
    }

    #[test]
    fn test_compute_tangents_single_triangle() {
        let mut mesh = CpuMesh::default();
        // Triangle on XZ plane with normals pointing up
        mesh.vertices.push(MeshVertex::from_arrays(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0], // Will be recomputed
            [0.0, 0.0],
        ));
        mesh.vertices.push(MeshVertex::from_arrays(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0],
        ));
        mesh.vertices.push(MeshVertex::from_arrays(
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 1.0],
        ));
        mesh.indices = vec![0, 1, 2];

        compute_tangents(&mut mesh);

        // Verify tangents were computed (not zero)
        for v in &mesh.vertices {
            let tan = Vec3::from_array([v.tangent[0], v.tangent[1], v.tangent[2]]);
            let len = tan.length();
            assert!(len > 0.99 && len < 1.01, "Tangent should be unit length");
        }
    }

    #[test]
    fn test_mesh_vertex_pod_traits() {
        // Verify Pod/Zeroable traits compile
        let _v: MeshVertex = bytemuck::Zeroable::zeroed();
        let _bytes = bytemuck::bytes_of(&_v);
    }

    #[test]
    fn test_cpu_mesh_clone() {
        let mut mesh1 = CpuMesh::default();
        mesh1.vertices.push(MeshVertex::from_arrays(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
        mesh1.indices.push(0);

        let mesh2 = mesh1.clone();
        assert_eq!(mesh1.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh1.indices.len(), mesh2.indices.len());
    }

    #[test]
    fn test_compute_tangents_single_vertex_degenerate() {
        // EDGE CASE: Single vertex referenced by triangle (degenerate)
        let mut mesh = CpuMesh::default();
        mesh.vertices.push(MeshVertex::from_arrays(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.5, 0.5],
        ));
        mesh.indices = vec![0, 0, 0]; // All same vertex (degenerate triangle)

        compute_tangents(&mut mesh);

        // Should not crash, tangent should remain valid (normalized or identity)
        let tan = Vec3::from_array([
            mesh.vertices[0].tangent[0],
            mesh.vertices[0].tangent[1],
            mesh.vertices[0].tangent[2],
        ]);
        let len = tan.length();
        assert!(len.is_finite(), "Tangent length should be finite");
    }
}
