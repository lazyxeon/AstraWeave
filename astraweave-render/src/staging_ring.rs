//! Per-frame ring buffer for transient GPU allocations.
//!
//! Provides fast sub-allocations from a pre-allocated GPU buffer, recycling
//! space automatically after a configurable number of frames-in-flight.
//! Eliminates per-frame `create_buffer_init()` calls for uniform and storage
//! data that changes every frame.
//!
//! # Usage
//! ```ignore
//! let mut ring = StagingRing::new(&device, 4 * 1024 * 1024, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
//! // Each frame:
//! ring.begin_frame();
//! let alloc = ring.allocate(std::mem::size_of::<MyUniform>() as u64, 256).unwrap();
//! queue.write_buffer(ring.buffer(), alloc.offset, bytemuck::bytes_of(&my_uniform));
//! // Use alloc.offset as a dynamic offset or bind sub-range
//! ```

use std::collections::VecDeque;

/// Minimum alignment for uniform buffer offsets (wgpu spec: 256 bytes).
pub const MIN_UNIFORM_ALIGN: u64 = 256;

/// Default staging ring capacity (4 MiB).
pub const DEFAULT_RING_SIZE: u64 = 4 * 1024 * 1024;

/// Maximum number of frames-in-flight before space is recycled.
const MAX_FRAMES_IN_FLIGHT: usize = 3;

/// Result of a sub-allocation from the ring buffer.
#[derive(Debug, Clone, Copy)]
pub struct SubAllocation {
    /// Byte offset into the ring buffer.
    pub offset: u64,
    /// Actual size allocated (may be larger than requested due to alignment).
    pub size: u64,
}

/// Per-frame snapshot recording how much of the ring was consumed.
#[derive(Debug, Clone, Copy)]
struct FrameRecord {
    /// Cursor position at the start of this frame.
    #[allow(dead_code)]
    start: u64,
    /// Cursor position at the end of this frame.
    end: u64,
}

/// A ring buffer backed by a single GPU buffer for fast per-frame sub-allocations.
///
/// Space is advanced linearly each frame and recycled once the corresponding
/// GPU work has completed (after `MAX_FRAMES_IN_FLIGHT` frames).
pub struct StagingRing {
    buffer: wgpu::Buffer,
    capacity: u64,
    /// Current write cursor into the ring.
    cursor: u64,
    /// Oldest byte offset that is still in use by in-flight GPU work.
    tail: u64,
    /// Per-frame records for tracking when regions become available.
    frame_records: VecDeque<FrameRecord>,
    /// Cursor at the start of the current (not-yet-submitted) frame.
    frame_start: u64,
    /// Total bytes allocated this frame (for diagnostics).
    frame_bytes: u64,
    /// High-water mark across all frames (for diagnostics).
    peak_bytes: u64,
}

impl StagingRing {
    /// Create a new ring buffer with the given capacity and usage flags.
    ///
    /// `usage` must include `COPY_DST` (for `queue.write_buffer`). Typically
    /// also includes `UNIFORM` or `STORAGE` depending on use case.
    pub fn new(device: &wgpu::Device, capacity: u64, usage: wgpu::BufferUsages) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_ring"),
            size: capacity,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            capacity,
            cursor: 0,
            tail: 0,
            frame_records: VecDeque::with_capacity(MAX_FRAMES_IN_FLIGHT + 1),
            frame_start: 0,
            frame_bytes: 0,
            peak_bytes: 0,
        }
    }

    /// Begin a new frame. Recycles old frames that have finished GPU execution.
    ///
    /// Call once at the start of each frame, before any `allocate()` calls.
    pub fn begin_frame(&mut self) {
        // Close out the previous frame (if any work was done).
        if self.cursor != self.frame_start {
            self.frame_records.push_back(FrameRecord {
                start: self.frame_start,
                end: self.cursor,
            });
        }

        // Retire the oldest frame if we've exceeded frames-in-flight.
        while self.frame_records.len() > MAX_FRAMES_IN_FLIGHT {
            if let Some(old) = self.frame_records.pop_front() {
                self.tail = old.end;
            }
        }

        self.frame_start = self.cursor;
        if self.frame_bytes > self.peak_bytes {
            self.peak_bytes = self.frame_bytes;
        }
        self.frame_bytes = 0;
    }

    /// Allocate a region of the given `size` with the specified byte `alignment`.
    ///
    /// Returns `None` if the ring is full (would overlap in-flight data).
    /// The returned [`SubAllocation`] contains the byte offset to use with
    /// `queue.write_buffer()` and bind group descriptors.
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<SubAllocation> {
        let align = alignment.max(1);
        let aligned_cursor = (self.cursor + align - 1) & !(align - 1);
        let end = aligned_cursor + size;

        if self.cursor >= self.tail {
            // Cursor is ahead of tail — check if we have room before wrapping.
            if end <= self.capacity {
                self.cursor = end;
                self.frame_bytes += end - aligned_cursor;
                return Some(SubAllocation {
                    offset: aligned_cursor,
                    size,
                });
            }
            // Try wrapping to the beginning.
            let wrapped_start = 0u64;
            let wrapped_end = wrapped_start + size;
            if wrapped_end <= self.tail || self.tail == 0 && self.frame_records.is_empty() {
                self.cursor = wrapped_end;
                self.frame_bytes += size;
                return Some(SubAllocation {
                    offset: wrapped_start,
                    size,
                });
            }
        } else {
            // Cursor is behind tail (wrapped around) — check space between.
            if end <= self.tail {
                self.cursor = end;
                self.frame_bytes += end - aligned_cursor;
                return Some(SubAllocation {
                    offset: aligned_cursor,
                    size,
                });
            }
        }

        None // Ring is full.
    }

    /// Get a reference to the backing GPU buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Current capacity in bytes.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Bytes allocated in the current (unsubmitted) frame.
    pub fn frame_bytes(&self) -> u64 {
        self.frame_bytes
    }

    /// Peak bytes allocated in any single frame (high-water mark).
    pub fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }

    /// Number of in-flight frame records.
    pub fn frames_in_flight(&self) -> usize {
        self.frame_records.len()
    }

    /// Reset the ring completely, discarding all in-flight tracking.
    ///
    /// Only safe to call when no GPU work referencing the ring is pending.
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.tail = 0;
        self.frame_start = 0;
        self.frame_bytes = 0;
        self.frame_records.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub device for unit tests — we test logic only, not GPU buffer creation.
    /// Full integration tests require a wgpu device and are in renderer_tests.rs.

    #[test]
    fn sub_allocation_alignment() {
        // Verify alignment math without a real GPU device.
        let align = 256u64;
        let cursor = 100u64;
        let aligned = (cursor + align - 1) & !(align - 1);
        assert_eq!(aligned, 256);

        let cursor2 = 256u64;
        let aligned2 = (cursor2 + align - 1) & !(align - 1);
        assert_eq!(aligned2, 256);

        let cursor3 = 257u64;
        let aligned3 = (cursor3 + align - 1) & !(align - 1);
        assert_eq!(aligned3, 512);
    }

    #[test]
    fn frame_record_size() {
        assert_eq!(
            std::mem::size_of::<FrameRecord>(),
            16,
            "FrameRecord should be 2 × u64"
        );
    }
}
