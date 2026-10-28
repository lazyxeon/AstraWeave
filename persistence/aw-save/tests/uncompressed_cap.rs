//! A compressed `.awsv` payload under the 4 MiB cap can still name an
//! uncompressed LZ4 size above 16 MiB. `decompress_size_prepended` reserves
//! that size before it looks at the compressed bytes. The loader must reject
//! the prefix first.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use aw_save::{decode_awsv_bytes, MAX_AWSV_COMPRESSED_BYTES, MAX_AWSV_UNCOMPRESSED_BYTES};

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct PeakAlloc;

fn note_alloc(size: usize) {
    let now = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
    let mut peak = PEAK.load(Ordering::Relaxed);
    while now > peak {
        match PEAK.compare_exchange_weak(peak, now, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(seen) => peak = seen,
        }
    }
}

fn note_free(size: usize) {
    CURRENT.fetch_sub(size, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for PeakAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            note_alloc(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            note_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note_free(layout.size());
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            if new_size > layout.size() {
                note_alloc(new_size - layout.size());
            } else {
                note_free(layout.size() - new_size);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOC: PeakAlloc = PeakAlloc;

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Header plus a 4-byte LZ4 size prefix. `data_len` stays inside the
/// compressed cap. The prefix names one byte past the uncompressed cap.
fn hostile_uncompressed_prefix() -> Vec<u8> {
    let declared = u32::try_from(MAX_AWSV_UNCOMPRESSED_BYTES + 1).expect("size fits u32");
    let payload = declared.to_le_bytes();
    assert!(payload.len() <= MAX_AWSV_COMPRESSED_BYTES);
    let crc = crc32_ieee(&payload);
    let mut bytes = Vec::with_capacity(16 + payload.len());
    bytes.extend_from_slice(b"ASVS");
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.push(1);
    bytes.push(0);
    bytes.extend_from_slice(&(u32::try_from(payload.len()).expect("len")).to_le_bytes());
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes
}

fn peak_during(body: impl FnOnce()) -> usize {
    let baseline = CURRENT.load(Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    body();
    PEAK.load(Ordering::Relaxed).saturating_sub(baseline)
}

#[test]
fn declared_uncompressed_size_above_the_cap_is_rejected_before_lz4_allocates() {
    assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);

    let bytes = hostile_uncompressed_prefix();
    let data_len = u32::from_le_bytes(bytes[8..12].try_into().expect("len")) as usize;
    let declared = u32::from_le_bytes(bytes[16..20].try_into().expect("prefix")) as usize;
    assert!(
        data_len <= MAX_AWSV_COMPRESSED_BYTES,
        "fixture must pass the compressed cap, data_len={data_len}"
    );
    assert!(
        declared > MAX_AWSV_UNCOMPRESSED_BYTES,
        "fixture must name an uncompressed size past the cap"
    );

    let mut decoded = Ok(Vec::new());
    let peak = peak_during(|| {
        decoded = decode_awsv_bytes(&bytes).map(|bundle| bundle.world.ecs_blob);
    });
    let err = decoded.expect_err("oversized uncompressed prefix must not load");
    let msg = format!("{err:#}");
    assert!(
        peak < 1024 * 1024,
        "decode reserved {peak} bytes before rejecting a {declared}-byte uncompressed prefix"
    );
    assert!(
        msg.contains("declared uncompressed size exceeds cap"),
        "uncompressed cap did not reject the prefix: {msg}"
    );
}
