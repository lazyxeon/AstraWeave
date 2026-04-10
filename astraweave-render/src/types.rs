use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tangent at location 12 (to match skinned variant convention)
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // uv at location 13
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

impl SkinnedVertex {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<SkinnedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tangent (xyz, w = handedness)
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // joints
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Uint16x4,
                },
                // weights
                wgpu::VertexAttribute {
                    offset: 56,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 3]; 3],
    pub color: [f32; 4],
    pub material_id: u32,
    pub _padding: [u32; 3], // 16-byte alignment
}

impl InstanceRaw {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model matrix (4x vec4)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // normal matrix (3x vec3 packed as Float32x3)
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 76,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 88,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 100,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // material_id (uint)
                wgpu::VertexAttribute {
                    offset: 116,
                    shader_location: 14,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: wgpu::Buffer,
    pub index_count: u32,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub transform: Mat4,
    pub color: [f32; 4],
    pub material_id: u32,
}

impl Instance {
    pub fn raw(&self) -> InstanceRaw {
        let model = self.transform;
        let normal = model.inverse().transpose();
        InstanceRaw {
            model: model.to_cols_array_2d(),
            normal_matrix: [
                normal.x_axis.truncate().to_array(),
                normal.y_axis.truncate().to_array(),
                normal.z_axis.truncate().to_array(),
            ],
            color: self.color,
            material_id: self.material_id,
            _padding: [0; 3],
        }
    }

    pub fn from_pos_scale_color(pos: Vec3, scale: Vec3, color: [f32; 4]) -> Self {
        let transform = Mat4::from_scale_rotation_translation(scale, glam::Quat::IDENTITY, pos);
        Self {
            transform,
            color,
            material_id: 0,
        }
    }
}

// ---- Phase 2 foundations: clustered lighting binning helpers ----
#[derive(Clone, Copy, Debug)]
pub struct ClusterDims {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Compute cluster index for a screen-space pixel and linear depth in view space.
/// Inputs:
/// - screen coords in [0,width) x [0,height)
/// - near/far planes
/// - dims: number of clusters in x/y/z
#[allow(clippy::too_many_arguments)]
pub fn cluster_index(
    px: u32,
    py: u32,
    width: u32,
    height: u32,
    depth: f32,
    near: f32,
    far: f32,
    dims: ClusterDims,
) -> u32 {
    let sx = (px as f32 * dims.x as f32 / width as f32).clamp(0.0, dims.x as f32 - 1.0) as u32;
    let sy = (py as f32 * dims.y as f32 / height as f32).clamp(0.0, dims.y as f32 - 1.0) as u32;
    // Logarithmic z slicing improves distribution
    let z_lin = ((depth - near) / (far - near)).clamp(0.0, 0.99999);
    let z_log = (z_lin * (dims.z as f32)).floor() as u32;
    sx + sy * dims.x + z_log * dims.x * dims.y
}

#[cfg(test)]
mod tests_cluster {
    use super::*;
    #[test]
    fn bins_within_bounds() {
        let dims = ClusterDims { x: 16, y: 9, z: 24 };
        let idx = cluster_index(100, 50, 1920, 1080, 5.0, 0.1, 100.0, dims);
        assert!(idx < dims.x * dims.y * dims.z);
    }
    #[test]
    fn corners_map_to_edges() {
        let d = ClusterDims { x: 8, y: 8, z: 8 };
        let i0 = cluster_index(0, 0, 800, 800, 0.1, 0.1, 100.0, d);
        let i1 = cluster_index(799, 799, 800, 800, 99.9, 0.1, 100.0, d);
        assert_ne!(i0, i1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_layout_attributes() {
        let layout = Vertex::layout();
        // 4 attributes: position, normal, tangent, uv
        assert_eq!(layout.attributes.len(), 4);
        // Verify shader locations
        assert_eq!(layout.attributes[0].shader_location, 0); // position
        assert_eq!(layout.attributes[1].shader_location, 1); // normal
        assert_eq!(layout.attributes[2].shader_location, 12); // tangent
        assert_eq!(layout.attributes[3].shader_location, 13); // uv
    }

    #[test]
    fn test_vertex_layout_stride() {
        let layout = Vertex::layout();
        // 3 floats (pos) + 3 floats (normal) + 4 floats (tangent) + 2 floats (uv) = 12 floats = 48 bytes
        assert_eq!(layout.array_stride, 48);
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);
    }

    #[test]
    fn test_skinned_vertex_layout_attributes() {
        let layout = SkinnedVertex::layout();
        // 6 attributes: position, normal, tangent, uv, joints, weights
        assert_eq!(layout.attributes.len(), 6);
        // Verify shader locations
        assert_eq!(layout.attributes[0].shader_location, 0); // position
        assert_eq!(layout.attributes[1].shader_location, 1); // normal
        assert_eq!(layout.attributes[2].shader_location, 12); // tangent
        assert_eq!(layout.attributes[3].shader_location, 13); // uv
        assert_eq!(layout.attributes[4].shader_location, 10); // joints
        assert_eq!(layout.attributes[5].shader_location, 11); // weights
    }

    #[test]
    fn test_skinned_vertex_layout_stride() {
        let layout = SkinnedVertex::layout();
        // 3f32 (pos) + 3f32 (norm) + 4f32 (tan) + 2f32 (uv) + 4u16 (joints) + 4f32 (weights)
        // = 12+12+16+8+8+16 = 72 bytes
        assert_eq!(layout.array_stride, 72);
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);
    }

    #[test]
    fn test_instance_raw_layout_attributes() {
        let layout = InstanceRaw::layout();
        // 9 attributes: 4x mat4, 3x mat3, color, material_id
        assert_eq!(layout.attributes.len(), 9);
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    }

    #[test]
    fn test_instance_raw_layout_stride() {
        let layout = InstanceRaw::layout();
        // 16 floats (model) + 9 floats (normal) + 4 floats (color) + 1 uint + 3 uint (padding)
        // = 16*4 + 9*4 + 4*4 + 4*4 = 64+36+16+16 = 132 bytes
        assert_eq!(layout.array_stride, 132);
    }

    #[test]
    fn test_instance_from_pos_scale_color() {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let scale = Vec3::new(2.0, 2.0, 2.0);
        let color = [1.0, 0.0, 0.0, 1.0];

        let inst = Instance::from_pos_scale_color(pos, scale, color);

        assert_eq!(inst.color, color);
        assert_eq!(inst.material_id, 0);

        // Verify position is correct
        let pos_from_mat = Vec3::new(
            inst.transform.w_axis.x,
            inst.transform.w_axis.y,
            inst.transform.w_axis.z,
        );
        assert!((pos_from_mat - pos).length() < 1e-5);
    }

    #[test]
    fn test_instance_raw_conversion() {
        let transform = Mat4::from_translation(Vec3::new(5.0, 10.0, 15.0));
        let color = [0.5, 0.5, 0.5, 1.0];
        let material_id = 42;

        let inst = Instance {
            transform,
            color,
            material_id,
        };

        let raw = inst.raw();

        assert_eq!(raw.color, color);
        assert_eq!(raw.material_id, material_id);
        assert_eq!(raw._padding, [0; 3]);

        // Verify model matrix is correct
        assert_eq!(raw.model[3][0], 5.0); // translation x
        assert_eq!(raw.model[3][1], 10.0); // translation y
        assert_eq!(raw.model[3][2], 15.0); // translation z
    }

    #[test]
    fn test_vertex_pod_traits() {
        // Verify Pod/Zeroable traits work (compile-time check)
        let _v: Vertex = bytemuck::Zeroable::zeroed();
        let _bytes = bytemuck::bytes_of(&_v);
    }

    #[test]
    fn test_skinned_vertex_pod_traits() {
        let _v: SkinnedVertex = bytemuck::Zeroable::zeroed();
        let _bytes = bytemuck::bytes_of(&_v);
    }

    #[test]
    fn test_instance_raw_pod_traits() {
        let _inst: InstanceRaw = bytemuck::Zeroable::zeroed();
        let _bytes = bytemuck::bytes_of(&_inst);
    }

    #[test]
    fn test_material_clone() {
        let mat1 = Material {
            color: [1.0, 0.5, 0.2, 1.0],
        };
        let mat2 = mat1.clone();
        assert_eq!(mat1.color, mat2.color);
    }

    #[test]
    fn test_cluster_dims_debug() {
        let dims = ClusterDims { x: 16, y: 9, z: 24 };
        let debug_str = format!("{:?}", dims);
        assert!(debug_str.contains("16"));
        assert!(debug_str.contains("9"));
        assert!(debug_str.contains("24"));
    }

    #[test]
    fn test_cluster_index_depth_progression() {
        let dims = ClusterDims { x: 8, y: 8, z: 8 };
        // Same pixel, increasing depth should increase Z slice
        let idx_near = cluster_index(400, 400, 800, 800, 1.0, 0.1, 100.0, dims);
        let idx_mid = cluster_index(400, 400, 800, 800, 50.0, 0.1, 100.0, dims);
        let idx_far = cluster_index(400, 400, 800, 800, 95.0, 0.1, 100.0, dims);

        // Indices should be different (depth slicing working)
        assert!(idx_near < idx_mid);
        assert!(idx_mid < idx_far);
    }

    #[test]
    fn test_cluster_index_clamping() {
        let dims = ClusterDims { x: 4, y: 4, z: 4 };
        // Out-of-bounds coordinates should clamp
        let idx = cluster_index(10000, 10000, 800, 800, 150.0, 0.1, 100.0, dims);
        assert!(
            idx < dims.x * dims.y * dims.z,
            "Index should be within bounds"
        );
    }
}
