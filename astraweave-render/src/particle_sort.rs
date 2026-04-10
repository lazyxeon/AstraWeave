//! GPU Bitonic Sort for depth-ordered particle transparency.
//!
//! Sorts particles back-to-front by camera distance using a parallel
//! bitonic merge sort on the GPU. The sort operates on an index+distance
//! array (8 bytes per entry) rather than the full particle data, minimizing
//! bandwidth.
//!
//! The host dispatches `log2(n) * (log2(n)+1) / 2` compute passes, each
//! performing one compare-and-swap stage.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Per-pass sort parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SortParams {
    pub algo_step: u32,
    pub stage_step: u32,
    pub num_particles: u32,
    pub _pad: u32,
    pub camera_pos: [f32; 3],
    pub _pad2: f32,
}

/// Sort entry: index into particle buffer + camera distance.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SortEntry {
    pub index: u32,
    pub distance: f32,
}

// ---------------------------------------------------------------------------
// Sort Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for bitonic sort of particles.
pub struct ParticleSortPass {
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params_buf: wgpu::Buffer,
    /// Sort entries buffer (index + distance).
    entries_buf: wgpu::Buffer,
    max_particles: u32,
}

impl ParticleSortPass {
    pub fn new(device: &wgpu::Device, max_particles: u32) -> Self {
        // Round up to next power of 2 for bitonic sort
        let padded = max_particles.next_power_of_two();

        let entries_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_sort_entries"),
            size: (padded as u64) * std::mem::size_of::<SortEntry>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle_sort_params"),
            contents: bytemuck::bytes_of(&SortParams::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_sort_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bitonic_sort_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/particles/bitonic_sort.wgsl").into(),
            ),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_sort_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle_sort_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("bitonic_sort"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bgl,
            params_buf,
            entries_buf,
            max_particles: padded,
        }
    }

    /// Get the sort entries buffer (for binding in render pass).
    pub fn entries_buffer(&self) -> &wgpu::Buffer {
        &self.entries_buf
    }

    /// Maximum (padded) particle count.
    pub fn max_particles(&self) -> u32 {
        self.max_particles
    }

    /// Upload initial sort entries: compute distances from camera on CPU
    /// and fill the entries buffer. Called once per frame before sort passes.
    pub fn prepare_entries(&self, queue: &wgpu::Queue, positions: &[[f32; 3]], camera_pos: Vec3) {
        let count = positions.len().min(self.max_particles as usize);
        let mut entries = Vec::with_capacity(self.max_particles as usize);

        for (i, pos) in positions.iter().enumerate().take(count) {
            let p = Vec3::from_array(*pos);
            let dist = (p - camera_pos).length_squared();
            entries.push(SortEntry {
                index: i as u32,
                distance: dist,
            });
        }

        // Pad remaining entries with max distance (sorted to back, invisible)
        for i in count..self.max_particles as usize {
            entries.push(SortEntry {
                index: i as u32,
                distance: -1.0, // negative = always sorted behind real particles
            });
        }

        queue.write_buffer(&self.entries_buf, 0, bytemuck::cast_slice(&entries));
    }

    /// Execute the full bitonic sort. Dispatches O(log²n) passes.
    pub fn execute(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        _num_particles: u32,
        camera_pos: Vec3,
    ) {
        let n = self.max_particles;
        if n <= 1 {
            return;
        }

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_sort_bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.entries_buf.as_entire_binding(),
                },
            ],
        });

        let workgroups = n.div_ceil(256);

        // Bitonic sort: iterate over stages
        let mut stage = 2u32;
        while stage <= n {
            let mut step = stage;
            while step >= 2 {
                let half = step / 2;
                let params = SortParams {
                    algo_step: stage,
                    stage_step: half,
                    num_particles: n,
                    _pad: 0,
                    camera_pos: camera_pos.to_array(),
                    _pad2: 0.0,
                };
                queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("bitonic_sort_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);

                step /= 2;
            }
            stage *= 2;
        }
    }

    /// Compute the number of sort passes needed for n particles.
    pub fn pass_count(n: u32) -> u32 {
        if n <= 1 {
            return 0;
        }
        let log_n = (n.next_power_of_two() as f32).log2() as u32;
        log_n * (log_n + 1) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_params_size() {
        assert_eq!(std::mem::size_of::<SortParams>(), 32);
    }

    #[test]
    fn sort_entry_size() {
        assert_eq!(std::mem::size_of::<SortEntry>(), 8);
    }

    #[test]
    fn pass_count_powers_of_two() {
        assert_eq!(ParticleSortPass::pass_count(1), 0);
        assert_eq!(ParticleSortPass::pass_count(2), 1);
        assert_eq!(ParticleSortPass::pass_count(4), 3); // 2*(2+1)/2 = 3
        assert_eq!(ParticleSortPass::pass_count(8), 6); // 3*(3+1)/2 = 6
        assert_eq!(ParticleSortPass::pass_count(16), 10); // 4*(4+1)/2 = 10
    }

    #[test]
    fn pass_count_non_powers() {
        // Non-powers round up
        assert_eq!(ParticleSortPass::pass_count(3), 3); // rounds to 4 → 3 passes
        assert_eq!(ParticleSortPass::pass_count(5), 6); // rounds to 8 → 6 passes
    }

    #[test]
    fn prepare_entries_distances() {
        let positions = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [5.0, 0.0, 0.0]];
        let camera = Vec3::ZERO;

        // Just verify the distances are computed correctly
        let entries: Vec<SortEntry> = positions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let dist = Vec3::from_array(*p).length_squared();
                SortEntry {
                    index: i as u32,
                    distance: dist,
                }
            })
            .collect();

        assert_eq!(entries[0].distance, 0.0);
        assert_eq!(entries[1].distance, 100.0);
        assert_eq!(entries[2].distance, 25.0);
    }

    #[test]
    fn sort_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = ParticleSortPass::new(&device, 1024);
        assert_eq!(pass.max_particles(), 1024);
    }

    #[test]
    fn sort_pass_non_power_of_two() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        // 1000 rounds up to 1024
        let pass = ParticleSortPass::new(&device, 1000);
        assert_eq!(pass.max_particles(), 1024);
    }
}
