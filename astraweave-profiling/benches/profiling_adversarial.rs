//! Adversarial Profiling Benchmarks
//!
//! Stress testing for Tracy profiling macros, zone overhead, and instrumentation.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::type_complexity)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;
use std::time::Instant;

// ============================================================================
// LOCAL TYPES (Mirror astraweave-profiling API)
// ============================================================================

/// Simulated profiling zone
#[derive(Clone, Debug)]
struct Zone {
    name: &'static str,
    start: Instant,
    color: u32,
    depth: u32,
}

impl Zone {
    fn new(name: &'static str, color: u32, depth: u32) -> Self {
        Self {
            name,
            start: Instant::now(),
            color,
            depth,
        }
    }

    fn elapsed_ns(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }
}

/// Thread-local profiling context
#[derive(Default)]
struct ProfilingContext {
    zone_stack: Vec<Zone>,
    completed_zones: Vec<(String, u64, u32)>, // (name, duration_ns, depth)
    frame_count: u64,
}

impl ProfilingContext {
    fn push_zone(&mut self, name: &'static str, color: u32) {
        let depth = self.zone_stack.len() as u32;
        self.zone_stack.push(Zone::new(name, color, depth));
    }

    fn pop_zone(&mut self) {
        if let Some(zone) = self.zone_stack.pop() {
            self.completed_zones
                .push((zone.name.to_string(), zone.elapsed_ns(), zone.depth));
        }
    }

    fn frame_mark(&mut self) {
        self.frame_count += 1;
        // In real Tracy, this would signal frame boundary
    }
}

/// Plot data for real-time graphs
#[derive(Clone, Debug)]
struct PlotData {
    name: String,
    values: Vec<(u64, f64)>, // (timestamp, value)
    min: f64,
    max: f64,
}

impl PlotData {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: Vec::new(),
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    fn add(&mut self, timestamp: u64, value: f64) {
        self.values.push((timestamp, value));
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
}

/// Message log entry
#[derive(Clone, Debug)]
struct Message {
    text: String,
    timestamp: u64,
    color: u32,
}

/// Memory allocation tracking
#[derive(Clone, Debug)]
struct AllocationEvent {
    ptr: u64,
    size: usize,
    name: Option<String>,
    is_alloc: bool,
    timestamp: u64,
}

/// Lock annotation for contention profiling
#[derive(Clone, Debug)]
struct LockEvent {
    name: String,
    address: u64,
    is_acquire: bool,
    is_try: bool,
    success: bool,
    timestamp: u64,
    thread_id: u64,
}

// ============================================================================
// CATEGORY 1: ZONE OPERATIONS
// ============================================================================

fn bench_zone_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_adversarial/zone_operations");

    // Test 1: Zone creation overhead
    group.bench_function("zone_creation_100000", |bencher| {
        bencher.iter(|| {
            let zones: Vec<Zone> = (0..100000)
                .map(|i| {
                    Zone::new(
                        "benchmark_zone",
                        0xFF0000 + (i % 256) as u32,
                        (i % 10) as u32,
                    )
                })
                .collect();

            std_black_box(zones.len())
        });
    });

    // Test 2: Push/pop cycle
    group.bench_function("push_pop_cycle_50000", |bencher| {
        let mut ctx = ProfilingContext::default();

        bencher.iter(|| {
            for _ in 0..50000 {
                ctx.push_zone("outer_zone", 0xFF0000);
                ctx.push_zone("inner_zone", 0x00FF00);
                ctx.pop_zone();
                ctx.pop_zone();
            }

            std_black_box(ctx.completed_zones.len())
        });
    });

    // Test 3: Nested zone hierarchy
    group.bench_function("nested_zones_depth_20", |bencher| {
        let mut ctx = ProfilingContext::default();

        bencher.iter(|| {
            for _ in 0..1000 {
                // Create deep nesting
                for depth in 0..20 {
                    ctx.push_zone("nested_zone", 0xFF0000 + depth * 0x10);
                }

                // Pop all
                for _ in 0..20 {
                    ctx.pop_zone();
                }
            }

            std_black_box(ctx.completed_zones.len())
        });
    });

    // Test 4: Zone with data collection
    group.bench_function("zone_data_collection_10000", |bencher| {
        let mut ctx = ProfilingContext::default();

        bencher.iter(|| {
            for i in 0..10000 {
                ctx.push_zone("data_zone", 0xFF0000);

                // Simulate some work that generates data
                let work_result: u64 = (0..100).map(|j| (i + j) as u64).sum();
                std_black_box(work_result);

                ctx.pop_zone();
            }

            // Aggregate results
            let total_ns: u64 = ctx.completed_zones.iter().map(|(_, ns, _)| ns).sum();
            std_black_box(total_ns)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 2: FRAME MARKING
// ============================================================================

fn bench_frame_marking(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_adversarial/frame_marking");

    // Test 1: Simple frame marks
    group.bench_function("frame_marks_10000", |bencher| {
        let mut ctx = ProfilingContext::default();

        bencher.iter(|| {
            for _ in 0..10000 {
                ctx.frame_mark();
            }

            std_black_box(ctx.frame_count)
        });
    });

    // Test 2: Frame marks with zone context
    group.bench_function("frame_marks_with_zones_5000", |bencher| {
        let mut ctx = ProfilingContext::default();

        bencher.iter(|| {
            for _ in 0..5000 {
                ctx.push_zone("frame", 0xFF0000);

                ctx.push_zone("update", 0x00FF00);
                ctx.pop_zone();

                ctx.push_zone("render", 0x0000FF);
                ctx.pop_zone();

                ctx.pop_zone();
                ctx.frame_mark();
            }

            std_black_box(ctx.frame_count)
        });
    });

    // Test 3: Frame time calculation
    group.bench_function("frame_time_calculation_1000", |bencher| {
        let frame_times: Vec<u64> = (0..1000)
            .map(|i| 16_000_000 + (i % 5_000_000)) // 16ms ± variation
            .collect();

        bencher.iter(|| {
            // Calculate rolling FPS
            let window_size = 60;
            let fps_values: Vec<f64> = frame_times
                .windows(window_size)
                .map(|window| {
                    let avg_ns: f64 =
                        window.iter().map(|&t| t as f64).sum::<f64>() / window.len() as f64;
                    1_000_000_000.0 / avg_ns
                })
                .collect();

            let min_fps = fps_values.iter().cloned().fold(f64::MAX, f64::min);
            let max_fps = fps_values.iter().cloned().fold(f64::MIN, f64::max);
            let avg_fps: f64 = fps_values.iter().sum::<f64>() / fps_values.len() as f64;

            std_black_box((min_fps, max_fps, avg_fps))
        });
    });

    // Test 4: Frame budget tracking
    group.bench_function("frame_budget_tracking_5000", |bencher| {
        let zone_timings: Vec<Vec<(&str, u64)>> = (0..5000)
            .map(|i| {
                vec![
                    ("update", 2_000_000 + (i % 500_000)),
                    ("physics", 3_000_000 + (i % 1_000_000)),
                    ("ai", 1_500_000 + (i % 300_000)),
                    ("render", 8_000_000 + (i % 2_000_000)),
                    ("ui", 500_000 + (i % 200_000)),
                ]
            })
            .collect();

        let budget_ns = 16_666_666u64; // 60 FPS

        bencher.iter(|| {
            let budget_analysis: Vec<(u64, bool, Vec<(&str, f64)>)> = zone_timings
                .iter()
                .map(|frame| {
                    let total: u64 = frame.iter().map(|(_, t)| t).sum();
                    let over_budget = total > budget_ns;

                    let percentages: Vec<(&str, f64)> = frame
                        .iter()
                        .map(|(name, time)| (*name, *time as f64 / budget_ns as f64 * 100.0))
                        .collect();

                    (total, over_budget, percentages)
                })
                .collect();

            let over_budget_count = budget_analysis.iter().filter(|(_, over, _)| *over).count();
            std_black_box(over_budget_count)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 3: PLOT DATA
// ============================================================================

fn bench_plot_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_adversarial/plot_data");

    // Test 1: Plot value recording
    group.bench_function("plot_recording_50000", |bencher| {
        let mut plot = PlotData::new("test_metric");

        bencher.iter(|| {
            for i in 0..50000 {
                let value = (i as f64 * 0.1).sin() * 100.0;
                plot.add(i as u64, value);
            }

            std_black_box((plot.min, plot.max, plot.values.len()))
        });
    });

    // Test 2: Multiple plots
    group.bench_function("multiple_plots_10_x_5000", |bencher| {
        let plot_names = [
            "fps",
            "frame_time",
            "memory",
            "cpu",
            "gpu",
            "entities",
            "draw_calls",
            "triangles",
            "textures",
            "audio",
        ];

        bencher.iter(|| {
            let mut plots: Vec<PlotData> =
                plot_names.iter().map(|name| PlotData::new(name)).collect();

            for i in 0..5000 {
                for (idx, plot) in plots.iter_mut().enumerate() {
                    let value = ((i + idx) as f64 * 0.1).sin() * 100.0 + idx as f64 * 10.0;
                    plot.add(i as u64, value);
                }
            }

            let total_values: usize = plots.iter().map(|p| p.values.len()).sum();
            std_black_box(total_values)
        });
    });

    // Test 3: Plot downsampling
    group.bench_function("plot_downsampling_100000", |bencher| {
        let values: Vec<(u64, f64)> = (0..100000)
            .map(|i| (i as u64, (i as f64 * 0.01).sin() * 100.0))
            .collect();

        let target_points = 1000usize;

        bencher.iter(|| {
            let bucket_size = values.len() / target_points;

            let downsampled: Vec<(u64, f64, f64, f64)> = values
                .chunks(bucket_size)
                .map(|chunk| {
                    let min = chunk.iter().map(|(_, v)| *v).fold(f64::MAX, f64::min);
                    let max = chunk.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max);
                    let avg: f64 = chunk.iter().map(|(_, v)| *v).sum::<f64>() / chunk.len() as f64;
                    let timestamp = chunk[chunk.len() / 2].0;

                    (timestamp, min, max, avg)
                })
                .collect();

            std_black_box(downsampled.len())
        });
    });

    // Test 4: Plot statistics
    group.bench_function("plot_statistics_10000", |bencher| {
        let values: Vec<f64> = (0..10000)
            .map(|i| (i as f64 * 0.01).sin() * 100.0 + 50.0)
            .collect();

        bencher.iter(|| {
            let count = values.len();
            let sum: f64 = values.iter().sum();
            let mean = sum / count as f64;

            let variance: f64 =
                values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;
            let std_dev = variance.sqrt();

            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let median = sorted[count / 2];
            let p95 = sorted[(count as f64 * 0.95) as usize];
            let p99 = sorted[(count as f64 * 0.99) as usize];

            std_black_box((mean, std_dev, median, p95, p99))
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 4: MESSAGE LOGGING
// ============================================================================

fn bench_message_logging(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_adversarial/message_logging");

    // Test 1: Message creation
    group.bench_function("message_creation_50000", |bencher| {
        bencher.iter(|| {
            let messages: Vec<Message> = (0..50000)
                .map(|i| Message {
                    text: format!("Event {} occurred at location {}", i, i % 100),
                    timestamp: i as u64,
                    color: 0xFFFFFF,
                })
                .collect();

            std_black_box(messages.len())
        });
    });

    // Test 2: Formatted message with values
    group.bench_function("formatted_messages_10000", |bencher| {
        let events: Vec<(&str, u64, f32)> = (0..10000)
            .map(|i| {
                let event_type = ["spawn", "update", "destroy", "collision", "damage"][i % 5];
                let entity_id = i as u64;
                let value = i as f32 * 0.1;
                (event_type, entity_id, value)
            })
            .collect();

        bencher.iter(|| {
            let messages: Vec<Message> = events
                .iter()
                .enumerate()
                .map(|(i, (event_type, entity_id, value))| Message {
                    text: format!("{}: entity {} = {:.2}", event_type, entity_id, value),
                    timestamp: i as u64,
                    color: match *event_type {
                        "spawn" => 0x00FF00,
                        "destroy" => 0xFF0000,
                        "collision" => 0xFFFF00,
                        "damage" => 0xFF8000,
                        _ => 0xFFFFFF,
                    },
                })
                .collect();

            std_black_box(messages.len())
        });
    });

    // Test 3: Message filtering
    group.bench_function("message_filtering_20000", |bencher| {
        let messages: Vec<Message> = (0..20000)
            .map(|i| Message {
                text: format!(
                    "[{}] Message {}",
                    ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"][i % 5],
                    i
                ),
                timestamp: i as u64,
                color: match i % 5 {
                    0 => 0xFF0000, // ERROR
                    1 => 0xFF8000, // WARN
                    2 => 0xFFFFFF, // INFO
                    3 => 0x808080, // DEBUG
                    _ => 0x404040, // TRACE
                },
            })
            .collect();

        bencher.iter(|| {
            // Filter to ERROR and WARN only
            let filtered: Vec<&Message> = messages
                .iter()
                .filter(|m| m.text.starts_with("[ERROR]") || m.text.starts_with("[WARN]"))
                .collect();

            std_black_box(filtered.len())
        });
    });

    // Test 4: Message ring buffer
    group.bench_function("message_ring_buffer_100000", |bencher| {
        let max_size = 10000usize;
        let mut buffer: Vec<Message> = Vec::with_capacity(max_size);
        let mut write_pos = 0usize;

        bencher.iter(|| {
            for i in 0..100000 {
                let msg = Message {
                    text: format!("Event {}", i),
                    timestamp: i as u64,
                    color: 0xFFFFFF,
                };

                if buffer.len() < max_size {
                    buffer.push(msg);
                } else {
                    buffer[write_pos] = msg;
                }

                write_pos = (write_pos + 1) % max_size;
            }

            std_black_box(buffer.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 5: MEMORY PROFILING
// ============================================================================

fn bench_memory_profiling(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_adversarial/memory_profiling");

    // Test 1: Allocation tracking
    group.bench_function("allocation_tracking_50000", |bencher| {
        bencher.iter(|| {
            let events: Vec<AllocationEvent> = (0..50000)
                .map(|i| AllocationEvent {
                    ptr: 0x1000_0000 + (i * 0x100) as u64,
                    size: 64 + (i % 1024) * 16,
                    name: Some(format!("alloc_{}", i % 100)),
                    is_alloc: i % 3 != 0, // 2/3 alloc, 1/3 free
                    timestamp: i as u64,
                })
                .collect();

            // Track live allocations
            let mut live: HashMap<u64, usize> = HashMap::new();
            let mut peak_size = 0usize;
            let mut current_size = 0usize;

            for event in &events {
                if event.is_alloc {
                    live.insert(event.ptr, event.size);
                    current_size += event.size;
                    peak_size = peak_size.max(current_size);
                } else if let Some(size) = live.remove(&event.ptr) {
                    current_size -= size;
                }
            }

            std_black_box((live.len(), peak_size))
        });
    });

    // Test 2: Memory category tracking
    group.bench_function("category_tracking_20000", |bencher| {
        let allocations: Vec<(&str, usize)> = (0..20000)
            .map(|i| {
                let category = ["texture", "mesh", "audio", "ai", "physics", "ui"][i % 6];
                let size = match category {
                    "texture" => 1024 * 1024 + (i % 4096) * 1024,
                    "mesh" => 65536 + (i % 1024) * 64,
                    "audio" => 16384 + (i % 512) * 32,
                    "ai" => 4096 + (i % 256) * 16,
                    "physics" => 8192 + (i % 512) * 32,
                    "ui" => 2048 + (i % 128) * 16,
                    _ => 1024,
                };
                (category, size)
            })
            .collect();

        bencher.iter(|| {
            let mut by_category: HashMap<&str, (usize, usize)> = HashMap::new();

            for (category, size) in &allocations {
                let entry = by_category.entry(category).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += size;
            }

            // Sort by size
            let mut sorted: Vec<_> = by_category.iter().collect();
            sorted.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

            std_black_box(sorted.len())
        });
    });

    // Test 3: Leak detection
    group.bench_function("leak_detection_10000", |bencher| {
        let events: Vec<(u64, bool, u64)> = (0..10000)
            .map(|i| {
                let ptr = 0x1000_0000 + (i % 1000 * 0x100) as u64;
                let is_alloc = i % 4 != 0; // Intentionally unbalanced
                let timestamp = i as u64;
                (ptr, is_alloc, timestamp)
            })
            .collect();

        bencher.iter(|| {
            let mut allocations: HashMap<u64, u64> = HashMap::new(); // ptr -> alloc_time

            for (ptr, is_alloc, timestamp) in &events {
                if *is_alloc {
                    allocations.insert(*ptr, *timestamp);
                } else {
                    allocations.remove(ptr);
                }
            }

            // Find potential leaks (still allocated at end)
            let leak_count = allocations.len();
            let oldest_leak = allocations.values().min().copied();

            std_black_box((leak_count, oldest_leak))
        });
    });

    // Test 4: Fragmentation analysis
    group.bench_function("fragmentation_analysis_5000", |bencher| {
        let allocations: Vec<(u64, usize)> = (0..5000)
            .map(|i| {
                let addr = 0x1000_0000 + (i * 0x1000) as u64;
                let size = 256 + (i % 4) * 256; // Variable sizes
                (addr, size)
            })
            .collect();

        bencher.iter(|| {
            // Sort by address
            let mut sorted = allocations.clone();
            sorted.sort_by_key(|(addr, _)| *addr);

            // Calculate gaps
            let mut gaps: Vec<usize> = Vec::new();
            for i in 1..sorted.len() {
                let end_prev = sorted[i - 1].0 + sorted[i - 1].1 as u64;
                let start_curr = sorted[i].0;
                if start_curr > end_prev {
                    gaps.push((start_curr - end_prev) as usize);
                }
            }

            let total_gap: usize = gaps.iter().sum();
            let total_alloc: usize = sorted.iter().map(|(_, s)| s).sum();
            let fragmentation = total_gap as f64 / (total_gap + total_alloc) as f64;

            std_black_box(fragmentation)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 6: LOCK PROFILING
// ============================================================================

fn bench_lock_profiling(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_adversarial/lock_profiling");

    // Test 1: Lock event recording
    group.bench_function("lock_event_recording_50000", |bencher| {
        bencher.iter(|| {
            let events: Vec<LockEvent> = (0..50000)
                .map(|i| LockEvent {
                    name: format!("mutex_{}", i % 20),
                    address: 0x1000_0000 + (i % 20 * 0x100) as u64,
                    is_acquire: i % 2 == 0,
                    is_try: i % 10 == 0,
                    success: i % 10 != 5, // 90% success rate
                    timestamp: i as u64,
                    thread_id: (i % 8) as u64,
                })
                .collect();

            std_black_box(events.len())
        });
    });

    // Test 2: Contention analysis
    group.bench_function("contention_analysis_20000", |bencher| {
        let events: Vec<LockEvent> = (0..20000)
            .map(|i| LockEvent {
                name: format!("lock_{}", i % 10),
                address: 0x1000_0000 + (i % 10 * 0x100) as u64,
                is_acquire: true,
                is_try: false,
                success: true,
                timestamp: i as u64,
                thread_id: (i % 4) as u64,
            })
            .collect();

        bencher.iter(|| {
            let mut contention: HashMap<String, HashMap<u64, usize>> = HashMap::new();

            for event in &events {
                let lock_entry = contention.entry(event.name.clone()).or_default();
                *lock_entry.entry(event.thread_id).or_insert(0) += 1;
            }

            // Find highly contended locks (multiple threads accessing)
            let contended: Vec<(&String, usize)> = contention
                .iter()
                .map(|(name, threads)| (name, threads.len()))
                .filter(|(_, thread_count)| *thread_count > 1)
                .collect();

            std_black_box(contended.len())
        });
    });

    // Test 3: Hold time calculation
    group.bench_function("hold_time_calculation_10000", |bencher| {
        // Pairs of acquire/release events
        let events: Vec<(u64, bool, u64)> = (0..10000)
            .flat_map(|i| {
                let lock_addr = 0x1000_0000 + (i % 50 * 0x100) as u64;
                let acquire_time = i as u64 * 2;
                let release_time = acquire_time + 10 + (i % 100) as u64;
                vec![
                    (lock_addr, true, acquire_time),
                    (lock_addr, false, release_time),
                ]
            })
            .collect();

        bencher.iter(|| {
            let mut held: HashMap<u64, u64> = HashMap::new(); // addr -> acquire_time
            let mut hold_times: HashMap<u64, Vec<u64>> = HashMap::new();

            for (addr, is_acquire, timestamp) in &events {
                if *is_acquire {
                    held.insert(*addr, *timestamp);
                } else if let Some(acquire_time) = held.remove(addr) {
                    let duration = timestamp - acquire_time;
                    hold_times.entry(*addr).or_default().push(duration);
                }
            }

            // Calculate statistics per lock
            let stats: Vec<(u64, f64, u64)> = hold_times
                .iter()
                .map(|(addr, times)| {
                    let avg = times.iter().sum::<u64>() as f64 / times.len() as f64;
                    let max = *times.iter().max().unwrap_or(&0);
                    (*addr, avg, max)
                })
                .collect();

            std_black_box(stats.len())
        });
    });

    // Test 4: Deadlock detection simulation
    group.bench_function("deadlock_detection_1000", |bencher| {
        // Lock acquisition order per thread
        let acquisitions: Vec<(u64, Vec<u64>)> = (0..1000)
            .map(|i| {
                let thread_id = i as u64;
                let locks: Vec<u64> = (0..5)
                    .map(|j| {
                        let base = if i % 2 == 0 { j } else { 4 - j }; // Alternate order
                        0x1000_0000 + base as u64 * 0x100
                    })
                    .collect();
                (thread_id, locks)
            })
            .collect();

        bencher.iter(|| {
            // Build lock order graph
            let mut lock_order: HashMap<u64, Vec<u64>> = HashMap::new(); // lock -> locks acquired after

            for (_, locks) in &acquisitions {
                for i in 0..locks.len() {
                    for j in (i + 1)..locks.len() {
                        lock_order.entry(locks[i]).or_default().push(locks[j]);
                    }
                }
            }

            // Simple cycle detection
            let mut potential_deadlocks = 0;
            for (lock, after) in &lock_order {
                for target in after {
                    if let Some(target_after) = lock_order.get(target) {
                        if target_after.contains(lock) {
                            potential_deadlocks += 1;
                        }
                    }
                }
            }

            std_black_box(potential_deadlocks)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_zone_operations,
    bench_frame_marking,
    bench_plot_data,
    bench_message_logging,
    bench_memory_profiling,
    bench_lock_profiling,
);

criterion_main!(benches);
