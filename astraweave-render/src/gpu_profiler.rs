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

/// How a begin/end pair was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampKind {
    /// `end > begin` and the span is a positive number of milliseconds.
    Measured,
    /// Both ticks were written and they are equal. The pass was shorter than
    /// one timestamp tick (or the driver collapsed the pair). Not a failure.
    Empty,
    /// Both ticks are 0. The query was resolved but never written.
    Unwritten,
    /// `end < begin`. Not a wrap of a full counter in one pass.
    Inverted,
    /// The device period is not positive, or a positive tick span underflows.
    BadPeriod,
}

/// Per-pass GPU timing result.
#[derive(Debug, Clone)]
pub struct PassTiming {
    pub name: String,
    /// Milliseconds for [`TimestampKind::Measured`]. Zero for every other kind.
    pub duration_ms: f32,
    pub begin_tick: u64,
    pub end_tick: u64,
    pub kind: TimestampKind,
}

/// Pass sum plus the optional whole-frame bracket.
#[derive(Debug, Clone)]
pub struct GpuStampSummary {
    /// Sum of measured passes other than the `frame` bracket.
    pub pass_sum_ms: f32,
    /// Encoder begin/end around `draw_into`, when that pair was measured.
    pub frame_ms: Option<f32>,
    pub empty: Vec<String>,
    pub report: String,
}

fn format_stamp_report(period_ns: f32, results: &[PassTiming]) -> String {
    let mut lines = vec![format!("timestamp_period_ns={period_ns}")];
    for timing in results {
        lines.push(format!(
            "pass={} kind={:?} begin={} end={} duration_ms={:.6}",
            timing.name, timing.kind, timing.begin_tick, timing.end_tick, timing.duration_ms
        ));
    }
    if results.is_empty() {
        lines.push("pass=(none)".to_string());
    }
    lines.join("\n")
}

fn summarize_stamps(period_ns: f32, results: &[PassTiming]) -> Result<GpuStampSummary, String> {
    let report = format_stamp_report(period_ns, results);
    if results.is_empty() {
        return Err(format!("no GPU timestamp samples\n{report}"));
    }
    let mut pass_sum_ms = 0.0f32;
    let mut frame_ms = None;
    let mut empty = Vec::new();
    let mut bad = Vec::new();
    for timing in results {
        match timing.kind {
            TimestampKind::Measured if timing.name == "frame" => {
                frame_ms = Some(timing.duration_ms);
            }
            TimestampKind::Measured => {
                pass_sum_ms += timing.duration_ms;
            }
            TimestampKind::Empty => empty.push(timing.name.clone()),
            TimestampKind::Unwritten | TimestampKind::Inverted | TimestampKind::BadPeriod => {
                bad.push(format!("{} {:?}", timing.name, timing.kind));
            }
        }
    }
    if !bad.is_empty() {
        return Err(format!(
            "GPU timestamp read failed ({})\n{report}",
            bad.join(", ")
        ));
    }
    if !strictly_positive(pass_sum_ms) && frame_ms.is_none() {
        return Err(format!(
            "no pass recorded a positive GPU duration\n{report}"
        ));
    }
    Ok(GpuStampSummary {
        pass_sum_ms,
        frame_ms,
        empty,
        report,
    })
}

fn strictly_positive(value: f32) -> bool {
    matches!(value.partial_cmp(&0.0), Some(std::cmp::Ordering::Greater))
}

/// Classify a raw pair. Equal non-zero ticks are [`TimestampKind::Empty`].
pub fn classify_timestamp(begin: u64, end: u64, period_ns: f32) -> (TimestampKind, f32) {
    match timestamp_duration_ms(begin, end, period_ns) {
        Ok(ms) => (TimestampKind::Measured, ms),
        Err("timestamp queries were not written") => (TimestampKind::Unwritten, 0.0),
        Err("timestamp end is before begin") => (TimestampKind::Inverted, 0.0),
        Err("GPU duration is zero or negative") if begin == end => (TimestampKind::Empty, 0.0),
        Err(_) => (TimestampKind::BadPeriod, 0.0),
    }
}

/// Convert a begin/end timestamp pair into milliseconds.
///
/// A zero period, an unwritten pair (both ticks 0), a NaN, or a non-positive
/// span is not a measurement. Equal non-zero ticks are a zero span, not an
/// unread query.
pub fn timestamp_duration_ms(begin: u64, end: u64, period_ns: f32) -> Result<f32, &'static str> {
    if !strictly_positive(period_ns) {
        return Err("timestamp period is not positive");
    }
    if begin == 0 && end == 0 {
        return Err("timestamp queries were not written");
    }
    if end < begin {
        return Err("timestamp end is before begin");
    }
    let ms = ((end - begin) as f64) * f64::from(period_ns) / 1_000_000.0;
    if !strictly_positive(ms as f32) {
        return Err("GPU duration is zero or negative");
    }
    Ok(ms as f32)
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
    /// `true` when that callback received `Ok`.
    map_ok: Arc<AtomicBool>,
    /// Why the last readback could not be used.
    last_error: Option<&'static str>,
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
            map_ok: Arc::new(AtomicBool::new(false)),
            last_error: None,
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
        self.last_error = None;
        self.map_ready.store(false, Ordering::Release);
        self.map_ok.store(false, Ordering::Release);
        let ready = self.map_ready.clone();
        let ok = self.map_ok.clone();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            ok.store(result.is_ok(), Ordering::Release);
            ready.store(true, Ordering::Release);
        });
    }

    /// Wait until the mapped timestamp buffer is ready, then decode it.
    pub fn block_on_readback(&mut self, device: &wgpu::Device) -> Result<(), &'static str> {
        let _ = device.poll(wgpu::PollType::Wait);
        if !self.poll_readback(device) {
            return Err(self
                .last_error
                .unwrap_or("timestamp readback did not complete"));
        }
        Ok(())
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

        if !self.map_ready.load(Ordering::Acquire) {
            // Still mapped. end_frame will not copy into this buffer.
            return false;
        }
        if !self.map_ok.load(Ordering::Acquire) {
            self.readback_pending = false;
            self.last_error = Some("timestamp buffer map failed");
            self.latest_results.clear();
            return false;
        }

        let slice = self.readback_buf.slice(..);
        let data = slice.get_mapped_range();
        let timestamps: &[u64] =
            bytemuck::cast_slice(&data[..self.next_query as usize * std::mem::size_of::<u64>()]);

        self.latest_results.clear();
        for (name, begin_idx, end_idx) in &self.pass_queries {
            let t0 = timestamps[*begin_idx as usize];
            let t1 = timestamps[*end_idx as usize];
            let (kind, duration_ms) = classify_timestamp(t0, t1, self.timestamp_period);
            self.latest_results.push(PassTiming {
                name: (*name).to_string(),
                duration_ms,
                begin_tick: t0,
                end_tick: t1,
                kind,
            });
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

    /// Nanoseconds per GPU tick. Zero means the device did not report a period.
    pub fn timestamp_period_ns(&self) -> f32 {
        self.timestamp_period
    }

    /// One line per pass: name, kind, raw ticks, and the device period.
    pub fn timestamp_report(&self) -> String {
        format_stamp_report(self.timestamp_period, &self.latest_results)
    }

    /// Measured passes, excluding the `frame` bracket.
    ///
    /// [`TimestampKind::Empty`] is listed and left out of the sum. Unwritten,
    /// inverted, and bad-period pairs are failures and the error includes
    /// [`Self::timestamp_report`].
    pub fn validated_stamp(&self) -> Result<GpuStampSummary, String> {
        summarize_stamps(self.timestamp_period, &self.latest_results)
    }

    /// Sum of measured passes other than `frame`.
    pub fn validated_total_ms(&self) -> Result<f32, String> {
        self.validated_stamp().map(|summary| summary.pass_sum_ms)
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

    #[test]
    fn zero_or_negative_gpu_duration_is_rejected() {
        assert!(timestamp_duration_ms(0, 0, 1.0).is_err());
        assert!(timestamp_duration_ms(10, 10, 1.0).is_err());
        assert!(timestamp_duration_ms(20, 10, 1.0).is_err());
        assert!(timestamp_duration_ms(10, 20, 0.0).is_err());
        assert!(timestamp_duration_ms(10, 20, -1.0).is_err());
        assert!(timestamp_duration_ms(10, 20, f32::NAN).is_err());
        let ms = timestamp_duration_ms(1_000_000, 2_000_000, 1.0).unwrap();
        assert!(ms > 0.0);
    }

    #[test]
    fn equal_ticks_are_empty_and_unwritten_still_fails() {
        assert_eq!(classify_timestamp(10, 10, 1.0).0, TimestampKind::Empty);
        assert_eq!(classify_timestamp(0, 0, 1.0).0, TimestampKind::Unwritten);
        assert_eq!(classify_timestamp(20, 10, 1.0).0, TimestampKind::Inverted);
        let (kind, ms) = classify_timestamp(1_000_000, 2_000_000, 1.0);
        assert_eq!(kind, TimestampKind::Measured);
        assert!(ms > 0.0);

        let mut results = vec![
            sample("shadow_cascade_3", 40, 40, TimestampKind::Empty, 0.0),
            sample("main_render", 40, 1_040_000, TimestampKind::Measured, 1.0),
            sample("frame", 10, 2_010_000, TimestampKind::Measured, 2.0),
        ];
        let summary = summarize_stamps(1.0, &results).expect("empty shadow is not a failure");
        assert!((summary.pass_sum_ms - 1.0).abs() < 1e-6);
        assert_eq!(summary.frame_ms, Some(2.0));
        assert_eq!(summary.empty, vec!["shadow_cascade_3".to_string()]);

        results.push(sample("post_process", 0, 0, TimestampKind::Unwritten, 0.0));
        let err = summarize_stamps(1.0, &results).expect_err("unwritten fails");
        assert!(err.contains("post_process"));
        assert!(err.contains("begin=0"));
        assert!(err.contains("timestamp_period_ns=1"));
    }

    fn sample(
        name: &str,
        begin: u64,
        end: u64,
        kind: TimestampKind,
        duration_ms: f32,
    ) -> PassTiming {
        PassTiming {
            name: name.to_string(),
            duration_ms,
            begin_tick: begin,
            end_tick: end,
            kind,
        }
    }

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
