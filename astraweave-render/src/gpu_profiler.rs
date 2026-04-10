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
    pass_queries: Vec<(String, u32, u32)>,
    /// Most recent readback results (updated asynchronously).
    latest_results: Vec<PassTiming>,
    /// Whether a readback is currently pending (buffer mapped).
    readback_pending: bool,
    /// Signalled by the map callback when the readback buffer is ready.
    map_ready: Arc<AtomicBool>,
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
            latest_results: Vec::new(),
            readback_pending: false,
            map_ready: Arc::new(AtomicBool::new(false)),
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
    pub fn allocate_pass(&mut self, name: &str) -> Option<(u32, u32)> {
        if self.next_query + 2 > MAX_QUERIES {
            return None;
        }
        let begin = self.next_query;
        let end = self.next_query + 1;
        self.next_query += 2;
        self.pass_queries.push((name.to_owned(), begin, end));
        Some((begin, end))
    }

    /// Allocate a timestamp write pair for a render pass.
    /// Returns `RenderPassTimestampWrites` to embed in the pass descriptor,
    /// or `None` if we've run out of query slots.
    pub fn render_pass_timestamps(
        &mut self,
        name: &str,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if self.next_query + 2 > MAX_QUERIES {
            return None;
        }
        let begin = self.next_query;
        let end = self.next_query + 1;
        self.next_query += 2;
        self.pass_queries.push((name.to_owned(), begin, end));

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
        name: &str,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        if self.next_query + 2 > MAX_QUERIES {
            return None;
        }
        let begin = self.next_query;
        let end = self.next_query + 1;
        self.next_query += 2;
        self.pass_queries.push((name.to_owned(), begin, end));

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
        let flag = self.map_ready.clone();
        slice.map_async(wgpu::MapMode::Read, move |_result| {
            flag.store(true, Ordering::Release);
        });
    }

    /// Poll for completed readback and update `latest_results`.
    /// Returns `true` if new data is available.
    pub fn poll_readback(&mut self, device: &wgpu::Device) -> bool {
        if !self.readback_pending {
            return false;
        }
        // Non-blocking poll — drives the map callback.
        let _ = device.poll(wgpu::PollType::Poll);

        if !self.map_ready.load(Ordering::Acquire) {
            // Mapping not complete yet.
            return false;
        }

        let slice = self.readback_buf.slice(..);
        let data = slice.get_mapped_range();
        let timestamps: &[u64] =
            bytemuck::cast_slice(&data[..self.next_query as usize * std::mem::size_of::<u64>()]);

        let ns_per_tick = self.timestamp_period as f64;
        let mut results = Vec::with_capacity(self.pass_queries.len());

        for (name, begin_idx, end_idx) in &self.pass_queries {
            let t0 = timestamps[*begin_idx as usize];
            let t1 = timestamps[*end_idx as usize];
            let delta_ns = (t1.wrapping_sub(t0)) as f64 * ns_per_tick;
            results.push(PassTiming {
                name: name.clone(),
                duration_ms: (delta_ns / 1_000_000.0) as f32,
            });
        }

        drop(data);
        self.readback_buf.unmap();
        self.readback_pending = false;
        self.map_ready.store(false, Ordering::Release);
        self.latest_results = results;
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
