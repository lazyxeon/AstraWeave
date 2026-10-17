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

/// Copied at the end of a frame encoder. A dropped command buffer leaves the
/// readback at 0.
pub const FRAME_EXEC_MAGIC: u32 = 0xA1B2_C3D4;

/// Maximum number of timestamp pairs (begin + end per pass).
const MAX_PASSES: u32 = 16;
const MAX_QUERIES: u32 = MAX_PASSES * 2;
/// `device.poll(Wait)` blocks inside the driver for up to 60s and can fail
/// to return when a query resolve waits on work that is still queued.
const GPU_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Poll until the queue is idle, or fail after 5s. The `point` names the wait
/// in the error. This does not call `PollType::Wait`.
pub fn wait_for_gpu(device: &wgpu::Device, point: &str) -> Result<(), String> {
    let deadline = std::time::Instant::now() + GPU_WAIT;
    loop {
        match device.poll(wgpu::PollType::Poll) {
            Ok(wgpu::PollStatus::QueueEmpty | wgpu::PollStatus::WaitSucceeded) => return Ok(()),
            Ok(wgpu::PollStatus::Poll) => {
                if std::time::Instant::now() >= deadline {
                    return Err(format!(
                        "GPU wait timed out after 5s at {point}. The submission did not finish."
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(err) => return Err(format!("device.poll failed at {point}: {err}")),
        }
    }
}

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

/// One timestamp write or the resolve/copy that publishes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StampKind {
    Write,
    Resolve,
    Copy,
}

/// CPU record of which query set, index, and `queue.submit` a timestamp op used.
///
/// `submit == 0` means the command is recorded and not submitted yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StampRecord {
    pub kind: StampKind,
    pub query_set: u64,
    pub index: u32,
    pub label: &'static str,
    pub submit: u64,
    /// Address of the `CommandEncoder` that recorded this command.
    pub encoder: u64,
}

/// End index of a pair opened by [`GpuProfiler::begin_stamp`].
#[derive(Clone, Copy, Debug)]
pub struct OpenStamp {
    name: &'static str,
    end: u32,
}

const STAMP_CAP: usize = 48;

fn resource_token<T: std::hash::Hash>(kind: u8, value: &T) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    value.hash(&mut hasher);
    hasher.finish()
}

fn query_set_token(set: &wgpu::QuerySet) -> u64 {
    resource_token(1, set)
}

const fn empty_stamp() -> StampRecord {
    StampRecord {
        kind: StampKind::Write,
        query_set: 0,
        index: 0,
        label: "",
        submit: 0,
        encoder: 0,
    }
}

fn encoder_id(encoder: &wgpu::CommandEncoder) -> u64 {
    encoder as *const wgpu::CommandEncoder as u64
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
    /// Fixed log so a frame does not allocate to remember where writes went.
    stamps: [StampRecord; STAMP_CAP],
    stamp_len: u8,
    submit_serial: u64,
    /// Last resolve/copy, so a later read can be matched to the buffer it mapped.
    resolve_buffer: u64,
    resolve_offset: u64,
    copy_src: u64,
    copy_dst: u64,
    copy_src_offset: u64,
    copy_dst_offset: u64,
    copy_size: u64,
    mapped_buffer: u64,
    /// `none`, `pending`, `ok`, or `err`.
    map_status: &'static str,
    raw_ticks: [u64; 16],
    raw_len: u8,
    parsed_queries: u32,
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
            stamps: [empty_stamp(); STAMP_CAP],
            stamp_len: 0,
            submit_serial: 0,
            resolve_buffer: 0,
            resolve_offset: 0,
            copy_src: 0,
            copy_dst: 0,
            copy_src_offset: 0,
            copy_dst_offset: 0,
            copy_size: 0,
            mapped_buffer: 0,
            map_status: "none",
            raw_ticks: [0; 16],
            raw_len: 0,
            parsed_queries: 0,
        }
    }

    /// Replace the query set and both timestamp buffers.
    ///
    /// The self-test reads a set that has never been written. The frame path
    /// reuses one set, and the query count changes after the first frame
    /// (cluster and the far cascade drop out). A fresh set, resolve buffer,
    /// and staging buffer make one sample match that self-test.
    pub fn recreate(&mut self, device: &wgpu::Device) -> Result<(), &'static str> {
        if self.readback_pending {
            return Err("cannot recreate timestamp targets while the staging buffer is mapped");
        }
        self.query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu_profiler_queries"),
            ty: wgpu::QueryType::Timestamp,
            count: MAX_QUERIES,
        });
        let buf_size = (MAX_QUERIES as u64) * std::mem::size_of::<u64>() as u64;
        self.resolve_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_profiler_resolve"),
            size: buf_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        self.readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_profiler_readback"),
            size: buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        self.next_query = 0;
        self.pass_queries.clear();
        self.stamp_len = 0;
        self.latest_results.clear();
        self.map_status = "none";
        self.raw_len = 0;
        self.parsed_queries = 0;
        self.copy_size = 0;
        Ok(())
    }

    /// Reset per-frame state. Call at the start of each frame before recording passes.
    pub fn begin_frame(&mut self) {
        self.next_query = 0;
        self.pass_queries.clear();
        self.stamp_len = 0;
    }

    fn note_stamp(
        &mut self,
        kind: StampKind,
        index: u32,
        label: &'static str,
        submit: u64,
        encoder: u64,
    ) {
        let len = self.stamp_len as usize;
        if len >= STAMP_CAP {
            return;
        }
        self.stamps[len] = StampRecord {
            kind,
            query_set: query_set_token(&self.query_set),
            index,
            label,
            submit,
            encoder,
        };
        self.stamp_len += 1;
    }

    /// `write_timestamp` on `encoder` for the begin index of `name`.
    ///
    /// This is the self-test's write. Pass descriptors are not used, so the
    /// write cannot be dropped while the pass is built, and it lands in this
    /// encoder rather than a pass-local command buffer.
    pub fn begin_stamp(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        name: &'static str,
    ) -> Option<OpenStamp> {
        let (begin, end) = self.allocate_pass(name)?;
        self.note_stamp(StampKind::Write, begin, name, 0, encoder_id(encoder));
        let query_set = self.query_set.clone();
        encoder.write_timestamp(&query_set, begin);
        Some(OpenStamp { name, end })
    }

    /// Matching end `write_timestamp` for [`Self::begin_stamp`], on the same encoder.
    pub fn end_stamp(&mut self, encoder: &mut wgpu::CommandEncoder, stamp: OpenStamp) {
        self.note_stamp(
            StampKind::Write,
            stamp.end,
            stamp.name,
            0,
            encoder_id(encoder),
        );
        let query_set = self.query_set.clone();
        encoder.write_timestamp(&query_set, stamp.end);
    }

    fn assign_open_stamps(&mut self, submit: u64) {
        for stamp in &mut self.stamps[..self.stamp_len as usize] {
            if stamp.submit == 0 {
                stamp.submit = submit;
            }
        }
    }

    /// Query-set token, index, and submit id for the stamps recorded since
    /// [`Self::begin_frame`].
    pub fn stamp_trace(&self) -> &[StampRecord] {
        &self.stamps[..self.stamp_len as usize]
    }

    /// One line per write, plus the resolve and copy submits.
    pub fn submission_report(&self) -> String {
        let mut lines = Vec::with_capacity(self.stamp_len as usize + 1);
        lines.push(format!("query_set={}", query_set_token(&self.query_set)));
        for stamp in self.stamp_trace() {
            lines.push(format!(
                "submit={} encoder={} kind={:?} label={} index={} query_set={}",
                stamp.submit, stamp.encoder, stamp.kind, stamp.label, stamp.index, stamp.query_set
            ));
        }
        let mut encoder_ids = [0u64; STAMP_CAP];
        let mut encoder_count = 0usize;
        for stamp in self.stamp_trace() {
            if stamp.kind == StampKind::Copy || stamp.encoder == 0 {
                continue;
            }
            if !encoder_ids[..encoder_count].contains(&stamp.encoder) {
                encoder_ids[encoder_count] = stamp.encoder;
                encoder_count += 1;
            }
        }
        lines.push(format!("encoder_count={encoder_count}"));
        if self.stamp_len == 0 {
            lines.push("submit=(none)".to_string());
        }
        lines.push(format!(
            "resolve_buffer={} resolve_offset={}",
            self.resolve_buffer, self.resolve_offset
        ));
        lines.push(format!(
            "copy src={} src_offset={} dst={} dst_offset={} size={}",
            self.copy_src,
            self.copy_src_offset,
            self.copy_dst,
            self.copy_dst_offset,
            self.copy_size
        ));
        lines.push(format!(
            "mapped_buffer={} map={} parsed_queries={}",
            self.mapped_buffer, self.map_status, self.parsed_queries
        ));
        let shown = self.raw_len as usize;
        let raw = self.raw_ticks[..shown]
            .iter()
            .map(|tick| tick.to_string())
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!("raw_u64={raw}"));
        lines.join("\n")
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

    /// This frame wrote queries and the staging buffer is free to receive them.
    pub fn staging_copy_pending(&self) -> bool {
        self.next_query > 0 && !self.readback_pending
    }

    /// Mark the frame closed. [`Self::resolve_queries`] records the resolve
    /// on the write encoder before [`Self::submit_frame`] submits that buffer.
    pub fn end_frame(&self, _encoder: &mut wgpu::CommandEncoder) {
        if self.readback_pending && self.next_query > 0 {
            log::debug!(
                "gpu profiler skipped timestamp resolve; gpu_profiler_readback is still mapped"
            );
        }
    }

    /// Resolve written queries into the GPU buffer on `encoder`.
    ///
    /// `encoder` is the `CommandEncoder` that recorded this frame's
    /// `write_timestamp` calls. wgpu tracks query resets per encoder. A
    /// resolve on a different encoder reads queries that encoder never wrote,
    /// and NVIDIA returns 0 for them. The staging copy stays a later submission
    /// (wgpu#6406).
    pub fn resolve_queries(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.staging_copy_pending() {
            return;
        }
        if self
            .stamp_trace()
            .iter()
            .any(|stamp| stamp.kind == StampKind::Resolve)
        {
            return;
        }
        self.resolve_buffer = resource_token(2, &self.resolve_buf);
        self.resolve_offset = 0;
        let index = self.next_query;
        let recorded_on = encoder_id(encoder);
        self.note_stamp(StampKind::Resolve, index, "resolve", 0, recorded_on);
        encoder.resolve_query_set(&self.query_set, 0..index, &self.resolve_buf, 0);
    }

    /// Every `write_timestamp` and the one `resolve_query_set` share an encoder.
    ///
    /// A matching submit id is not enough: `queue.submit` can take two
    /// command buffers, and that id does not say which encoder recorded the
    /// resolve.
    fn assert_shared_write_encoder(&self) -> Result<(), String> {
        let mut resolve_encoder = 0u64;
        let mut resolve_count = 0u32;
        for stamp in self.stamp_trace() {
            if stamp.kind == StampKind::Resolve {
                resolve_encoder = stamp.encoder;
                resolve_count += 1;
            }
        }
        if resolve_count != 1 || resolve_encoder == 0 {
            return Err(format!(
                "expected one resolve_query_set on the write encoder, found {resolve_count}\n{}",
                self.submission_report()
            ));
        }
        for stamp in self.stamp_trace() {
            if stamp.kind != StampKind::Write {
                continue;
            }
            if stamp.encoder != resolve_encoder {
                return Err(format!(
                    "timestamp write {} index {} encoder {} != resolve encoder {}\n{}",
                    stamp.label,
                    stamp.index,
                    stamp.encoder,
                    resolve_encoder,
                    self.submission_report()
                ));
            }
        }
        Ok(())
    }

    /// Copy the resolved timestamps into the CPU staging buffer.
    ///
    /// Submit this encoder only after the write-and-resolve submission is idle
    /// (wgpu#6406). It is not the encoder that wrote the timestamps.
    pub fn copy_resolved(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.staging_copy_pending() {
            return;
        }
        let byte_count = (self.next_query as u64) * std::mem::size_of::<u64>() as u64;
        self.copy_src = resource_token(2, &self.resolve_buf);
        self.copy_dst = resource_token(3, &self.readback_buf);
        self.copy_src_offset = 0;
        self.copy_dst_offset = 0;
        self.copy_size = byte_count;
        encoder.copy_buffer_to_buffer(&self.resolve_buf, 0, &self.readback_buf, 0, byte_count);
    }

    /// Submit the command buffer that already contains the writes and the resolve.
    ///
    /// [`Self::resolve_queries`] must have run on that same `CommandEncoder`.
    /// Opening a second encoder for the resolve submits two Vulkan command
    /// buffers. wgpu resets queries per command buffer, so the resolve reads
    /// queries that buffer never wrote, and NVIDIA returns 0. The submit id
    /// is logical and does not show that split. The copy is a later submission
    /// (wgpu#6406). Resolving only after a wait of the writes deadlocks NVIDIA:
    /// `VK_QUERY_RESULT_WAIT_BIT` runs while those writes are still queued.
    /// Every wait is a non-blocking poll with a 5s limit.
    pub fn submit_frame(
        profiler: Option<&mut Self>,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        frame: wgpu::CommandBuffer,
    ) -> Result<(), String> {
        let Some(profiler) = profiler else {
            queue.submit(std::iter::once(frame));
            return wait_for_gpu(device, "frame submit");
        };
        if !profiler.staging_copy_pending() {
            queue.submit(std::iter::once(frame));
            return wait_for_gpu(device, "frame submit");
        }
        profiler.assert_shared_write_encoder()?;
        profiler.submit_serial += 1;
        let write_submit = profiler.submit_serial;
        profiler.assign_open_stamps(write_submit);
        queue.submit(std::iter::once(frame));
        wait_for_gpu(device, "timestamp write+resolve")?;
        let mut copy = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu profiler timestamp copy"),
        });
        let copy_encoder = encoder_id(&copy);
        profiler.copy_resolved(&mut copy);
        profiler.submit_serial += 1;
        let copy_submit = profiler.submit_serial;
        profiler.note_stamp(
            StampKind::Copy,
            profiler.next_query,
            "copy",
            copy_submit,
            copy_encoder,
        );
        queue.submit(std::iter::once(copy.finish()));
        wait_for_gpu(device, "timestamp staging copy")
    }

    /// Initiate an async readback of the timestamp buffer.
    /// Call after `queue.submit()`.
    pub fn request_readback(&mut self) {
        if self.next_query == 0 || self.readback_pending {
            return;
        }
        let slice = self.readback_buf.slice(..);
        self.mapped_buffer = resource_token(3, &self.readback_buf);
        self.map_status = "pending";
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
    ///
    /// Uses [`wait_for_gpu`]'s 5s poll, not `PollType::Wait`. The map callback
    /// only stores an atomic, so this thread can poll it.
    pub fn block_on_readback(&mut self, device: &wgpu::Device) -> Result<(), String> {
        if !self.readback_pending {
            return Err("timestamp readback was not requested".to_string());
        }
        let deadline = std::time::Instant::now() + GPU_WAIT;
        loop {
            if let Err(err) = device.poll(wgpu::PollType::Poll) {
                return Err(format!(
                    "device.poll failed at timestamp map readback: {err}"
                ));
            }
            if self.poll_readback(device) {
                return Ok(());
            }
            if !self.readback_pending {
                return Err(self
                    .last_error
                    .unwrap_or("timestamp readback did not complete")
                    .to_string());
            }
            if std::time::Instant::now() >= deadline {
                return Err(
                    "GPU wait timed out after 5s at timestamp map readback. The submission did not finish."
                        .to_string(),
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
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
            self.map_status = "err";
            self.last_error = Some("timestamp buffer map failed");
            self.latest_results.clear();
            return false;
        }

        let slice = self.readback_buf.slice(..);
        let data = slice.get_mapped_range();
        let raw_bytes = (16 * std::mem::size_of::<u64>()).min(data.len());
        let raw: &[u64] = bytemuck::cast_slice(&data[..raw_bytes]);
        self.raw_ticks = [0; 16];
        self.raw_len = raw.len() as u8;
        self.raw_ticks[..raw.len()].copy_from_slice(raw);
        self.parsed_queries = self.next_query;
        self.map_status = "ok";
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

    /// Write two encoder timestamps around a 1×1 clear, resolve them on that
    /// same encoder, then use [`Self::submit_frame`]. The copy is the next
    /// submission. `Ok` includes `encoder_count`.
    ///
    /// `Ok` is the stamp report. A `0, 0` pair is a failure. Equal non-zero
    /// ticks are [`TimestampKind::Empty`]: a trivial pass can be shorter than
    /// one timestamp tick, and that is not a failure.
    pub fn prove_readback(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<String, String> {
        let features = device.features();
        if !features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Err("timestamp self-test: device is missing TIMESTAMP_QUERY".to_string());
        }
        if !features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
            return Err(
                "timestamp self-test: device is missing TIMESTAMP_QUERY_INSIDE_ENCODERS, so write_timestamp cannot run"
                    .to_string(),
            );
        }
        let mut profiler = Self::new(device, queue);
        let period = profiler.timestamp_period_ns();
        if !strictly_positive(period) {
            return Err(format!(
                "timestamp self-test: timestamp_period_ns={period} is not positive"
            ));
        }
        profiler.begin_frame();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("timestamp self-test target"),
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
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut writes = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("timestamp self-test writes"),
        });
        let outer = profiler
            .begin_stamp(&mut writes, "self_test")
            .ok_or_else(|| {
                "timestamp self-test: no query slot for the encoder bracket".to_string()
            })?;
        let clear = profiler
            .begin_stamp(&mut writes, "self_test_clear")
            .ok_or_else(|| "timestamp self-test: no query slot for the clear pass".to_string())?;
        {
            let _pass = writes.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("timestamp self-test clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        profiler.end_stamp(&mut writes, clear);
        profiler.end_stamp(&mut writes, outer);
        profiler.resolve_queries(&mut writes);
        profiler.end_frame(&mut writes);

        let frame = writes.finish();
        Self::submit_frame(Some(&mut profiler), queue, device, frame)
            .map_err(|err| format!("timestamp self-test: {err}\ntimestamp_period_ns={period}"))?;
        profiler.request_readback();
        if let Err(err) = profiler.block_on_readback(device) {
            return Err(format!(
                "timestamp self-test: {err}\n{}",
                profiler.timestamp_report()
            ));
        }
        let report = profiler.timestamp_report();
        let unread: Vec<&str> = profiler
            .results()
            .iter()
            .filter(|timing| timing.begin_tick == 0 && timing.end_tick == 0)
            .map(|timing| timing.name.as_str())
            .collect();
        if !unread.is_empty() {
            return Err(format!(
                "timestamp self-test read zeros ({}); resolve_query_set must be on the write encoder and the copy is a later submission\n{report}\n{}",
                unread.join(", "),
                profiler.submission_report()
            ));
        }
        if profiler.results().is_empty() {
            return Err(format!("timestamp self-test read no slots\n{report}"));
        }
        Ok(format!("{report}\n{}", profiler.submission_report()))
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
        let adapter_features = adapter.features();
        if !adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let mut required = wgpu::Features::TIMESTAMP_QUERY;
        if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
            required |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }
        if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES) {
            required |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
        }
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("profiler-readback-test"),
            required_features: required,
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
            profiler.resolve_queries(&mut enc);
            profiler.end_frame(&mut enc);
            enc.finish()
        };
        let frame = record(&mut profiler);
        assert!(
            profiler.staging_copy_pending(),
            "the first frame must have queries to resolve"
        );
        GpuProfiler::submit_frame(Some(&mut profiler), &queue, &device, frame)
            .expect("first timestamp submit");
        profiler.request_readback();
        let frame = record(&mut profiler);
        assert!(
            !profiler.staging_copy_pending(),
            "staging copy must be skipped while gpu_profiler_readback is mapped"
        );
        queue.submit(std::iter::once(frame));
        let _ = device.poll(wgpu::PollType::Wait);
        let err = pollster::block_on(device.pop_error_scope());
        if let Some(err) = err {
            panic!("profiler readback validation: {err}");
        }
    }

    #[test]
    fn timestamp_self_test_reads_nonzero() {
        let Some((device, queue)) = timestamp_device() else {
            eprintln!("no TIMESTAMP_QUERY adapter; skipping profiler self-test");
            return;
        };
        if !device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            eprintln!("no TIMESTAMP_QUERY_INSIDE_ENCODERS; skipping profiler self-test");
            return;
        }
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let report = GpuProfiler::prove_readback(&device, &queue).unwrap_or_else(|err| {
            panic!("timestamp self-test read zeros or failed: {err}");
        });
        assert!(
            !report.contains("begin=0 end=0"),
            "self-test report still has an unwritten pair:\n{report}"
        );
        assert!(
            report.contains("encoder_count=1\n"),
            "self-test writes and resolve must share one encoder\n{report}"
        );
        let err = pollster::block_on(device.pop_error_scope());
        if let Some(err) = err {
            panic!("timestamp self-test validation: {err}\n{report}");
        }
    }
}
