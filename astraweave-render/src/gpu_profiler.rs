//! GPU timestamp profiler for per-pass performance analysis.
//!
//! Wraps wgpu's timestamp query mechanism to automatically capture timing for
//! each render/compute pass. Results are read back asynchronously and exposed
//! as per-pass millisecond durations.
//!
//! Requires `Features::TIMESTAMP_QUERY` on the device.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Maximum number of timestamp pairs (begin + end per pass).
const MAX_PASSES: u32 = 16;
const MAX_QUERIES: u32 = MAX_PASSES * 2;

/// Per-pass GPU timing result.
#[derive(Debug, Clone)]
pub struct PassTiming {
    pub name: String,
    pub duration_ms: f32,
}

/// Manages GPU timestamp queries across render/compute passes.
pub struct GpuProfiler {
    query_set: wgpu::QuerySet,
    /// Buffer for resolved query results (GPU-only, COPY_SRC).
    resolve_buf: wgpu::Buffer,
    /// Staging buffer for CPU readback (MAP_READ | COPY_DST).
    readback_buf: wgpu::Buffer,
    /// Nanoseconds per GPU timestamp tick.
    timestamp_period: f32,
    /// Next query index to allocate this frame.
    next_query: u32,
    /// Map from pass name → (begin_query, end_query) indices.
    /// Names are `'static` so a steady frame does not allocate a `String`.
    pass_queries: Vec<(&'static str, u32, u32)>,
    /// Most recent readback results (updated asynchronously).
    latest_results: Vec<PassTiming>,
    /// Whether a readback is currently pending (buffer mapped).
    readback_pending: bool,
    /// Signalled by the map callback when the readback buffer is ready.
    map_ready: Arc<AtomicBool>,
    /// The map callback failed. The buffer is not mapped and must not be read.
    map_failed: Arc<AtomicBool>,
}

impl GpuProfiler {
    /// Create a new GPU profiler. Only call when `TIMESTAMP_QUERY` is available.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu_profiler_queries"),
            ty: wgpu::QueryType::Timestamp,
            count: MAX_QUERIES,
        });

        let buf_size = (MAX_QUERIES as u64) * std::mem::size_of::<u64>() as u64;

        let resolve_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_profiler_resolve"),
            size: buf_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_profiler_readback"),
            size: buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let timestamp_period = queue.get_timestamp_period();

        Self {
            query_set,
            resolve_buf,
            readback_buf,
            timestamp_period,
            next_query: 0,
            pass_queries: Vec::with_capacity(MAX_PASSES as usize),
            latest_results: Vec::with_capacity(MAX_PASSES as usize),
            readback_pending: false,
            map_ready: Arc::new(AtomicBool::new(false)),
            map_failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Reset per-frame state. Call at the start of each frame before recording passes.
    pub fn begin_frame(&mut self) {
        self.next_query = 0;
        self.pass_queries.clear();
    }

    /// Expose the underlying query set for constructing timestamp writes
    /// separately from the allocation step. This enables borrow splitting
    /// in the Renderer: allocate indices via `&mut self`, then build
    /// `TimestampWrites` from `&self.query_set` without conflicting borrows.
    pub fn query_set(&self) -> &wgpu::QuerySet {
        &self.query_set
    }

    /// Allocate a begin/end query index pair for a named pass.
    /// Returns `(begin_index, end_index)` or `None` if slots are exhausted.
    /// The returned indices can be used with `query_set()` to construct
    /// `RenderPassTimestampWrites` or `ComputePassTimestampWrites` inline.
    pub fn allocate_pass(&mut self, name: &'static str) -> Option<(u32, u32)> {
        if self.next_query + 2 > MAX_QUERIES {
            return None;
        }
        let begin = self.next_query;
        let end = self.next_query + 1;
        self.next_query += 2;
        self.pass_queries.push((name, begin, end));
        Some((begin, end))
    }

    /// Allocate a timestamp write pair for a render pass.
    /// Returns `RenderPassTimestampWrites` to embed in the pass descriptor,
    /// or `None` if we've run out of query slots.
    pub fn render_pass_timestamps(
        &mut self,
        name: &'static str,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if self.next_query + 2 > MAX_QUERIES {
            return None;
        }
        let begin = self.next_query;
        let end = self.next_query + 1;
        self.next_query += 2;
        self.pass_queries.push((name, begin, end));

        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(begin),
            end_of_pass_write_index: Some(end),
        })
    }

    /// Allocate a timestamp write pair for a compute pass.
    /// Returns `ComputePassTimestampWrites` to embed in the pass descriptor,
    /// or `None` if we've run out of query slots.
    pub fn compute_pass_timestamps(
        &mut self,
        name: &'static str,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        if self.next_query + 2 > MAX_QUERIES {
            return None;
        }
        let begin = self.next_query;
        let end = self.next_query + 1;
        self.next_query += 2;
        self.pass_queries.push((name, begin, end));

        Some(wgpu::ComputePassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(begin),
            end_of_pass_write_index: Some(end),
        })
    }

    /// After all passes are recorded, resolve and copy timestamps.
    /// Call before `queue.submit()`.
    pub fn end_frame(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.next_query == 0 {
            return;
        }
        // A mapped MAP_READ buffer cannot be a copy destination. Skipping the
        // resolve keeps Queue::submit from reporting that gpu_profiler_readback
        // is still mapped. The next frame copies once poll_readback unmaps it.
        if self.readback_pending {
            log::debug!(
                "gpu profiler skipped timestamp copy; gpu_profiler_readback is still mapped"
            );
            return;
        }
        // Resolve timestamps into the GPU buffer.
        encoder.resolve_query_set(&self.query_set, 0..self.next_query, &self.resolve_buf, 0);
        // Copy to the CPU-readable staging buffer.
        let byte_count = (self.next_query as u64) * std::mem::size_of::<u64>() as u64;
        encoder.copy_buffer_to_buffer(&self.resolve_buf, 0, &self.readback_buf, 0, byte_count);
    }

    /// Initiate an async readback of the timestamp buffer.
    /// Call after `queue.submit()`.
    pub fn request_readback(&mut self) {
        if self.next_query == 0 || self.readback_pending {
            return;
        }
        let slice = self.readback_buf.slice(..);
        self.readback_pending = true;
        self.map_ready.store(false, Ordering::Release);
        self.map_failed.store(false, Ordering::Release);
        let ready = self.map_ready.clone();
        let failed = self.map_failed.clone();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            if result.is_ok() {
                ready.store(true, Ordering::Release);
            } else {
                failed.store(true, Ordering::Release);
            }
        });
    }

    /// Poll for completed readback and update `latest_results`.
    /// Returns `true` if new data is available.
    pub fn poll_readback(&mut self, device: &wgpu::Device) -> bool {
        if !self.readback_pending {
            return false;
        }
        // Non-blocking poll — drives the map callback.
        if device.poll(wgpu::PollType::Poll).is_err() {
            log::error!("gpu profiler device poll failed; not reading a mapped buffer");
            return false;
        }

        if self.map_failed.swap(false, Ordering::AcqRel) {
            // map_async failed, so the buffer is not mapped. Do not unmap or read it.
            self.readback_pending = false;
            self.map_ready.store(false, Ordering::Release);
            log::error!("gpu profiler readback map failed; leaving the buffer unmapped");
            return false;
        }

        if !self.map_ready.load(Ordering::Acquire) {
            // Still mapped. end_frame will not copy into this buffer.
            return false;
        }

        let slice = self.readback_buf.slice(..);
        let data = slice.get_mapped_range();
        let timestamps: &[u64] =
            bytemuck::cast_slice(&data[..self.next_query as usize * std::mem::size_of::<u64>()]);

        let ns_per_tick = self.timestamp_period as f64;
        let n = self.pass_queries.len();
        let same_slots = self.latest_results.len() == n
            && self
                .latest_results
                .iter()
                .zip(self.pass_queries.iter())
                .all(|(timing, (name, _, _))| timing.name == *name);
        if same_slots {
            for (timing, (_, begin_idx, end_idx)) in
                self.latest_results.iter_mut().zip(self.pass_queries.iter())
            {
                let t0 = timestamps[*begin_idx as usize];
                let t1 = timestamps[*end_idx as usize];
                let delta_ns = (t1.wrapping_sub(t0)) as f64 * ns_per_tick;
                timing.duration_ms = (delta_ns / 1_000_000.0) as f32;
            }
        } else {
            self.latest_results.clear();
            for (name, begin_idx, end_idx) in &self.pass_queries {
                let t0 = timestamps[*begin_idx as usize];
                let t1 = timestamps[*end_idx as usize];
                let delta_ns = (t1.wrapping_sub(t0)) as f64 * ns_per_tick;
                self.latest_results.push(PassTiming {
                    name: (*name).to_string(),
                    duration_ms: (delta_ns / 1_000_000.0) as f32,
                });
            }
        }

        drop(data);
        self.readback_buf.unmap();
        self.readback_pending = false;
        self.map_ready.store(false, Ordering::Release);
        true
    }

    /// Get the most recent per-pass timing results.
    pub fn results(&self) -> &[PassTiming] {
        &self.latest_results
    }

    /// Get total GPU time across all instrumented passes (ms).
    pub fn total_gpu_ms(&self) -> f32 {
        self.latest_results.iter().map(|r| r.duration_ms).sum()
    }

    /// Get per-pass timings as a sorted map (name → ms).
    pub fn results_map(&self) -> BTreeMap<String, f32> {
        self.latest_results
            .iter()
            .map(|r| (r.name.clone(), r.duration_ms))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        if !adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("profiler-readback-test"),
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: Default::default(),
        }))
        .ok()
    }

    #[test]
    fn timestamp_copy_skips_a_buffer_that_is_still_mapped() {
        let Some((device, queue)) = timestamp_device() else {
            eprintln!("no TIMESTAMP_QUERY adapter; skipping profiler map test");
            return;
        };
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut profiler = GpuProfiler::new(&device, &queue);
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("profiler target"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let record = |profiler: &mut GpuProfiler| {
            profiler.begin_frame();
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("profiler frame"),
            });
            let timestamps = profiler.render_pass_timestamps("clear");
            {
                let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("profiler clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: timestamps,
                    occlusion_query_set: None,
                });
            }
            profiler.end_frame(&mut enc);
            enc.finish()
        };
        queue.submit(std::iter::once(record(&mut profiler)));
        profiler.request_readback();
        queue.submit(std::iter::once(record(&mut profiler)));
        let _ = device.poll(wgpu::PollType::Wait);
        let err = pollster::block_on(device.pop_error_scope());
        if let Some(err) = err {
            panic!("profiler readback validation: {err}");
        }
    }
}
