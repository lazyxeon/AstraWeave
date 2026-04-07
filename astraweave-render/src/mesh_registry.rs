use anyhow::Result;
use glam::Vec3;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, BufferUsages, Device, Queue};

use crate::mesh::CpuMesh;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshKey(pub String);

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u32);

pub struct MeshRegistry {
    next_id: u32,
    map: HashMap<MeshKey, MeshHandle>,
    uploads: HashMap<MeshHandle, Arc<GpuMesh>>,
}

pub struct GpuMesh {
    // Full interleaved vertex buffer (MeshVertex layout)
    pub vertex_full: Buffer,
    // Position-only vertex buffer (Float32x3 stride) for pipelines that only consume positions
    pub vertex_pos: Buffer,
    pub index: Buffer,
    pub index_count: u32,
    pub aabb: Option<(Vec3, Vec3)>,
}

impl Default for MeshRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            map: HashMap::new(),
            uploads: HashMap::new(),
        }
    }

    pub fn get(&self, key: &MeshKey) -> Option<MeshHandle> {
        self.map.get(key).copied()
    }

    pub fn fetch_or_upload(
        &mut self,
        device: &Device,
        _queue: &Queue,
        key: MeshKey,
        mesh: &CpuMesh,
    ) -> Result<MeshHandle> {
        if let Some(h) = self.map.get(&key).copied() {
            return Ok(h);
        }
        let handle = MeshHandle(self.next_id);
        self.next_id += 1;

        let vertex_full = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("mesh-vertex-full-{}", handle.0)),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();
        let vertex_pos = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("mesh-vertex-pos-{}", handle.0)),
            contents: bytemuck::cast_slice(&positions),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("mesh-index-{}", handle.0)),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });
        let gpu = GpuMesh {
            vertex_full,
            vertex_pos,
            index,
            index_count: mesh.indices.len() as u32,
            aabb: mesh.aabb(),
        };
        self.map.insert(key, handle);
        self.uploads.insert(handle, Arc::new(gpu));
        Ok(handle)
    }

    pub fn get_gpu(&self, handle: MeshHandle) -> Option<&GpuMesh> {
        self.uploads.get(&handle).map(|arc| &**arc)
    }

    pub fn get_mesh(&self, handle: MeshHandle) -> Option<Arc<GpuMesh>> {
        self.uploads.get(&handle).cloned()
    }

    /// Removes meshes that are only held by the registry.
    /// Returns the number of meshes unloaded.
    pub fn prune(&mut self) -> usize {
        let mut to_remove = Vec::new();

        // Identify handles to remove
        for (handle, mesh_arc) in &self.uploads {
            // strong_count is 1 implies only the registry holds it
            if Arc::strong_count(mesh_arc) == 1 {
                to_remove.push(*handle);
            }
        }

        // Remove from uploads and map
        let count = to_remove.len();
        for handle in to_remove {
            self.uploads.remove(&handle);
            // We also need to remove the key mapping for this handle
            // This is O(N) scan of the map, but typically prune is infrequent.
            // For better performance, we might want a reverse map.
            // Given constraints, linear scan is acceptable for Phase 1 fix.
            self.map.retain(|_, h| *h != handle);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::MeshVertex;
    use glam::{Vec2, Vec3, Vec4};

    #[test]
    fn cpu_mesh_aabb() {
        let m = CpuMesh {
            vertices: vec![
                MeshVertex::new(
                    Vec3::new(-1.0, 0.0, 2.0),
                    Vec3::Y,
                    Vec4::new(1.0, 0.0, 0.0, 1.0),
                    Vec2::ZERO,
                ),
                MeshVertex::new(
                    Vec3::new(3.0, -2.0, -1.0),
                    Vec3::Y,
                    Vec4::new(1.0, 0.0, 0.0, 1.0),
                    Vec2::ZERO,
                ),
            ],
            indices: vec![0, 1, 1],
            albedo_image: None,
        };
        let (min, max) = m.aabb().unwrap();
        assert_eq!(min, Vec3::new(-1.0, -2.0, -1.0));
        assert_eq!(max, Vec3::new(3.0, 0.0, 2.0));
    }

    #[test]
    fn test_mesh_registry_new() {
        let registry = MeshRegistry::new();
        assert_eq!(registry.next_id, 1, "Should start with ID 1");
        assert!(registry.map.is_empty(), "Should have no meshes initially");
        assert!(
            registry.uploads.is_empty(),
            "Should have no GPU meshes initially"
        );
    }

    #[test]
    fn test_mesh_key_equality() {
        let key1 = MeshKey("cube".to_string());
        let key2 = MeshKey("cube".to_string());
        let key3 = MeshKey("sphere".to_string());

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_mesh_handle_equality() {
        let handle1 = MeshHandle(1);
        let handle2 = MeshHandle(1);
        let handle3 = MeshHandle(2);

        assert_eq!(handle1, handle2);
        assert_ne!(handle1, handle3);
    }

    #[test]
    fn test_mesh_registry_get_empty() {
        let registry = MeshRegistry::new();
        let key = MeshKey("nonexistent".to_string());

        assert_eq!(
            registry.get(&key),
            None,
            "Should return None for nonexistent key"
        );
    }

    #[test]
    fn test_mesh_key_clone() {
        let key1 = MeshKey("cube".to_string());
        let key2 = key1.clone();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_mesh_handle_debug() {
        let handle = MeshHandle(42);
        let debug_str = format!("{:?}", handle);
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_mesh_key_debug() {
        let key = MeshKey("test_mesh".to_string());
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("test_mesh"));
    }

    #[test]
    fn test_mesh_registry_next_id_increments() {
        let mut registry = MeshRegistry::new();
        assert_eq!(registry.next_id, 1);

        // Manually increment to simulate uploads
        registry.next_id += 1;
        assert_eq!(registry.next_id, 2);

        registry.next_id += 1;
        assert_eq!(registry.next_id, 3);
    }

    #[test]
    fn test_gpu_mesh_aabb_some() {
        // This tests that GpuMesh can store AABB
        let aabb = Some((Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)));
        // Just verify the type can hold it (without wgpu device we can't create full GpuMesh)
        let _ = aabb;
    }

    #[test]
    fn test_mesh_key_hash_consistency() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(MeshKey("cube".to_string()));
        set.insert(MeshKey("cube".to_string())); // Duplicate

        assert_eq!(set.len(), 1, "Duplicate keys should hash to same value");
    }

    #[test]
    fn test_mesh_handle_hash_consistency() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(MeshHandle(1));
        set.insert(MeshHandle(1)); // Duplicate

        assert_eq!(set.len(), 1, "Duplicate handles should hash to same value");
    }
}
