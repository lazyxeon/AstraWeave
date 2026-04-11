// Week 5 Action 19: GPU Instancing Module
// Batches identical meshes to reduce draw calls and improve rendering performance

use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;
use wgpu;

/// Instance transform data (sent to GPU)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    /// Model matrix (4x4, column-major)
    pub model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn from_transform(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let model = Mat4::from_scale_rotation_translation(scale, rotation, position);
        Self {
            model: model.to_cols_array_2d(),
        }
    }

    pub fn from_matrix(matrix: Mat4) -> Self {
        Self {
            model: matrix.to_cols_array_2d(),
        }
    }

    /// Returns the vertex buffer layout descriptor for instance data
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Model matrix column 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Model matrix column 1
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Model matrix column 2
                wgpu::VertexAttribute {
                    offset: 2 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Model matrix column 3
                wgpu::VertexAttribute {
                    offset: 3 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// High-level instance transform (CPU side)
#[derive(Debug, Clone)]
pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Instance {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw::from_transform(self.position, self.rotation, self.scale)
    }
}

/// Batch of instances for the same mesh
#[derive(Debug)]
pub struct InstanceBatch {
    /// Mesh identifier (e.g., mesh ID, asset path hash)
    pub mesh_id: u64,
    /// Instance transforms
    pub instances: Vec<Instance>,
    /// GPU buffer for instance data
    pub buffer: Option<wgpu::Buffer>,
}

impl InstanceBatch {
    pub fn new(mesh_id: u64) -> Self {
        Self {
            mesh_id,
            instances: Vec::new(),
            buffer: None,
        }
    }

    pub fn add_instance(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub fn instance_count(&self) -> u32 {
        self.instances.len() as u32
    }

    /// Create or update GPU buffer with current instances.
    /// Uses grow-on-demand strategy: only allocates a new buffer when the
    /// current one is too small, and uses `queue.write_buffer()` otherwise.
    pub fn update_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.instances.is_empty() {
            return;
        }

        let instance_data: Vec<InstanceRaw> =
            self.instances.iter().map(|inst| inst.to_raw()).collect();

        let required_bytes = (instance_data.len() * std::mem::size_of::<InstanceRaw>()) as u64;

        // Grow-on-demand: allocate (with 2× rounding) only when capacity is exceeded.
        let needs_realloc = match &self.buffer {
            Some(buf) => buf.size() < required_bytes,
            None => true,
        };

        if needs_realloc {
            let alloc_size = required_bytes.next_power_of_two().max(256);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Instance Buffer (mesh {})", self.mesh_id)),
                size: alloc_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        // SAFETY: buffer is guaranteed Some — allocated in the `needs_realloc` branch above,
        // or pre-existing with sufficient capacity.
        queue.write_buffer(
            self.buffer
                .as_ref()
                .expect("instance buffer allocated above"),
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }

    /// Clear all instances (does not free GPU buffer immediately)
    pub fn clear(&mut self) {
        self.instances.clear();
    }
}

/// Manages instancing for all meshes in the scene
pub struct InstanceManager {
    /// Batches grouped by mesh ID
    batches: HashMap<u64, InstanceBatch>,
    /// Total number of instances across all batches
    total_instances: usize,
    /// Draw call reduction statistics
    draw_calls_saved: usize,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            batches: HashMap::new(),
            total_instances: 0,
            draw_calls_saved: 0,
        }
    }

    /// Add instance for a specific mesh
    pub fn add_instance(&mut self, mesh_id: u64, instance: Instance) {
        let batch = self
            .batches
            .entry(mesh_id)
            .or_insert_with(|| InstanceBatch::new(mesh_id));
        batch.add_instance(instance);
        self.total_instances += 1;
    }

    /// Add multiple instances for a mesh at once
    pub fn add_instances(&mut self, mesh_id: u64, instances: Vec<Instance>) {
        let count = instances.len();
        let batch = self
            .batches
            .entry(mesh_id)
            .or_insert_with(|| InstanceBatch::new(mesh_id));
        for instance in instances {
            batch.add_instance(instance);
        }
        self.total_instances += count;
    }

    /// Update all GPU buffers (call once per frame before rendering)
    pub fn update_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for batch in self.batches.values_mut() {
            batch.update_buffer(device, queue);
        }
        self.calculate_draw_call_savings();
    }

    /// Get batch for a specific mesh
    pub fn get_batch(&self, mesh_id: u64) -> Option<&InstanceBatch> {
        self.batches.get(&mesh_id)
    }

    /// Get mutable batch for a specific mesh
    pub fn get_batch_mut(&mut self, mesh_id: u64) -> Option<&mut InstanceBatch> {
        self.batches.get_mut(&mesh_id)
    }

    /// Iterate over all batches
    pub fn batches(&self) -> impl Iterator<Item = &InstanceBatch> {
        self.batches.values()
    }

    /// Clear all instances
    pub fn clear(&mut self) {
        self.batches.clear();
        self.total_instances = 0;
        self.draw_calls_saved = 0;
    }

    /// Get total number of instances
    pub fn total_instances(&self) -> usize {
        self.total_instances
    }

    /// Get number of unique meshes being instanced
    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    /// Get number of draw calls saved by instancing
    pub fn draw_calls_saved(&self) -> usize {
        self.draw_calls_saved
    }

    /// Calculate draw call reduction (internal)
    fn calculate_draw_call_savings(&mut self) {
        // Without instancing: 1 draw call per instance
        // With instancing: 1 draw call per batch
        let without_instancing = self.total_instances;
        let with_instancing = self.batches.len();
        self.draw_calls_saved = without_instancing.saturating_sub(with_instancing);
    }

    /// Get draw call reduction percentage
    pub fn draw_call_reduction_percent(&self) -> f32 {
        if self.total_instances == 0 {
            return 0.0;
        }
        (self.draw_calls_saved as f32 / self.total_instances as f32) * 100.0
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for generating common instance patterns
pub struct InstancePatternBuilder {
    instances: Vec<Instance>,
}

impl InstancePatternBuilder {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    /// Create a grid pattern of instances
    pub fn grid(mut self, rows: usize, cols: usize, spacing: f32) -> Self {
        for row in 0..rows {
            for col in 0..cols {
                let x = col as f32 * spacing;
                let z = row as f32 * spacing;
                self.instances.push(Instance::new(
                    Vec3::new(x, 0.0, z),
                    Quat::IDENTITY,
                    Vec3::ONE,
                ));
            }
        }
        self
    }

    /// Create a circle pattern of instances
    pub fn circle(mut self, count: usize, radius: f32) -> Self {
        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            let rotation = Quat::from_rotation_y(angle + std::f32::consts::PI);
            self.instances
                .push(Instance::new(Vec3::new(x, 0.0, z), rotation, Vec3::ONE));
        }
        self
    }

    /// Add random variation to positions
    pub fn with_position_jitter(mut self, amount: f32) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        for instance in &mut self.instances {
            instance.position.x += rng.random_range(-amount..amount);
            instance.position.z += rng.random_range(-amount..amount);
        }
        self
    }

    /// Add random variation to scales
    pub fn with_scale_variation(mut self, min: f32, max: f32) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        for instance in &mut self.instances {
            let scale = rng.random_range(min..max);
            instance.scale = Vec3::splat(scale);
        }
        self
    }

    /// Add random rotation around Y axis
    pub fn with_random_rotation_y(mut self) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        for instance in &mut self.instances {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            instance.rotation = Quat::from_rotation_y(angle);
        }
        self
    }

    pub fn build(self) -> Vec<Instance> {
        self.instances
    }
}

impl Default for InstancePatternBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_raw_size() {
        // 4x4 matrix = 16 floats = 64 bytes
        assert_eq!(std::mem::size_of::<InstanceRaw>(), 64);
    }

    #[test]
    fn test_instance_creation() {
        let instance = Instance::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::ONE);

        assert_eq!(instance.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(instance.scale, Vec3::ONE);
    }

    #[test]
    fn test_instance_to_raw() {
        let instance = Instance::identity();
        let raw = instance.to_raw();

        // Identity matrix should have 1s on diagonal
        assert_eq!(raw.model[0][0], 1.0);
        assert_eq!(raw.model[1][1], 1.0);
        assert_eq!(raw.model[2][2], 1.0);
        assert_eq!(raw.model[3][3], 1.0);
    }

    #[test]
    fn test_batch_management() {
        let mut batch = InstanceBatch::new(42);

        assert_eq!(batch.instance_count(), 0);

        batch.add_instance(Instance::identity());
        batch.add_instance(Instance::identity());

        assert_eq!(batch.instance_count(), 2);
        assert_eq!(batch.mesh_id, 42);
    }

    #[test]
    fn test_instance_manager() {
        let mut manager = InstanceManager::new();

        // Add instances for two different meshes
        manager.add_instance(1, Instance::identity());
        manager.add_instance(1, Instance::identity());
        manager.add_instance(2, Instance::identity());

        assert_eq!(manager.total_instances(), 3);
        assert_eq!(manager.batch_count(), 2);

        // 3 instances -> 2 batches = 1 draw call saved
        manager.calculate_draw_call_savings();
        assert_eq!(manager.draw_calls_saved(), 1);
    }

    #[test]
    fn test_draw_call_reduction() {
        let mut manager = InstanceManager::new();

        // 100 instances of same mesh
        for _ in 0..100 {
            manager.add_instance(1, Instance::identity());
        }

        manager.calculate_draw_call_savings();

        // Without instancing: 100 draw calls
        // With instancing: 1 draw call
        // Saved: 99 draw calls
        assert_eq!(manager.draw_calls_saved(), 99);

        let reduction = manager.draw_call_reduction_percent();
        assert!((reduction - 99.0).abs() < 0.01);
    }

    #[test]
    fn test_grid_pattern() {
        let instances = InstancePatternBuilder::new().grid(3, 3, 2.0).build();

        assert_eq!(instances.len(), 9); // 3x3 grid

        // Check corner positions
        assert_eq!(instances[0].position, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(instances[8].position, Vec3::new(4.0, 0.0, 4.0)); // (2*2, 0, 2*2)
    }

    #[test]
    fn test_circle_pattern() {
        let instances = InstancePatternBuilder::new().circle(8, 10.0).build();

        assert_eq!(instances.len(), 8);

        // All instances should be roughly 10 units from origin
        for instance in &instances {
            let distance = instance.position.length();
            assert!((distance - 10.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_pattern_with_variations() {
        let instances = InstancePatternBuilder::new()
            .grid(2, 2, 5.0)
            .with_scale_variation(0.8, 1.2)
            .with_random_rotation_y()
            .build();

        assert_eq!(instances.len(), 4);

        // All scales should be in range [0.8, 1.2]
        for instance in &instances {
            let scale = instance.scale.x; // Uniform scale
            assert!((0.8..=1.2).contains(&scale));
        }
    }

    #[test]
    fn test_batch_clearing() {
        let mut manager = InstanceManager::new();
        manager.add_instance(1, Instance::identity());
        manager.add_instance(2, Instance::identity());

        assert_eq!(manager.total_instances(), 2);

        manager.clear();

        assert_eq!(manager.total_instances(), 0);
        assert_eq!(manager.batch_count(), 0);
    }
}
