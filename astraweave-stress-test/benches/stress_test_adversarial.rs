//! Adversarial benchmarks for astraweave-stress-test
//!
//! Meta stress tests that stress test the stress testing infrastructure:
//! - Benchmark harness overhead measurement
//! - Statistical stability validation
//! - Memory allocation patterns under stress
//! - Thread contention scenarios
//! - Garbage collection pressure simulation
//! - System resource exhaustion patterns

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::VecDeque;
use std::hint::black_box as std_black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// LOCAL TYPE DEFINITIONS (Standalone benchmark - no crate imports)
// ============================================================================

/// Statistics collector for meta-benchmarking
#[derive(Clone, Debug, Default)]
struct BenchmarkStats {
    samples: Vec<f64>,
    sum: f64,
    sum_squared: f64,
    min: f64,
    max: f64,
}

impl BenchmarkStats {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            sum: 0.0,
            sum_squared: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    fn add_sample(&mut self, value: f64) {
        self.samples.push(value);
        self.sum += value;
        self.sum_squared += value * value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            0.0
        } else {
            self.sum / self.samples.len() as f64
        }
    }

    fn variance(&self) -> f64 {
        if self.samples.len() < 2 {
            0.0
        } else {
            let mean = self.mean();
            (self.sum_squared / self.samples.len() as f64) - (mean * mean)
        }
    }

    fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    fn coefficient_of_variation(&self) -> f64 {
        let mean = self.mean();
        if mean.abs() < 1e-10 {
            0.0
        } else {
            self.std_dev() / mean
        }
    }

    fn percentile(&mut self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p / 100.0) * (self.samples.len() - 1) as f64) as usize;
        self.samples[idx.min(self.samples.len() - 1)]
    }
}

/// Memory allocation tracker
#[allow(dead_code)]
struct AllocationTracker {
    allocations: Vec<(usize, Instant)>,
    deallocations: Vec<(usize, Instant)>,
    current_allocated: usize,
    peak_allocated: usize,
    total_allocated: usize,
}

#[allow(dead_code)]
impl AllocationTracker {
    fn new() -> Self {
        Self {
            allocations: Vec::new(),
            deallocations: Vec::new(),
            current_allocated: 0,
            peak_allocated: 0,
            total_allocated: 0,
        }
    }

    fn allocate(&mut self, size: usize) {
        self.allocations.push((size, Instant::now()));
        self.current_allocated += size;
        self.total_allocated += size;
        self.peak_allocated = self.peak_allocated.max(self.current_allocated);
    }

    fn deallocate(&mut self, size: usize) {
        self.deallocations.push((size, Instant::now()));
        self.current_allocated = self.current_allocated.saturating_sub(size);
    }

    fn allocation_rate(&self) -> f64 {
        if self.allocations.len() < 2 {
            return 0.0;
        }
        let first = self.allocations.first().unwrap().1;
        let last = self.allocations.last().unwrap().1;
        let duration = last.duration_since(first).as_secs_f64();
        if duration < 1e-9 {
            0.0
        } else {
            self.total_allocated as f64 / duration
        }
    }
}

/// Stress pattern generator
struct StressPatternGenerator {
    seed: u64,
    counter: u64,
}

impl StressPatternGenerator {
    fn new(seed: u64) -> Self {
        Self { seed, counter: 0 }
    }

    fn next_u64(&mut self) -> u64 {
        self.counter += 1;
        let mut x = self.seed.wrapping_add(self.counter);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    /// Generate burst pattern: periods of high activity followed by calm
    fn burst_pattern(&mut self, iteration: usize) -> bool {
        let phase = iteration % 100;
        // Burst for 20 iterations, then calm for 80
        phase < 20
    }

    /// Generate random walk pattern
    fn random_walk(&mut self, current: f64, volatility: f64) -> f64 {
        let delta = (self.next_f64() - 0.5) * 2.0 * volatility;
        (current + delta).max(0.0)
    }
}

/// Work unit for stress testing
#[allow(dead_code)]
struct WorkUnit {
    id: u64,
    data: Vec<u8>,
    priority: u32,
    created_at: Instant,
}

impl WorkUnit {
    fn new(id: u64, size: usize, priority: u32) -> Self {
        Self {
            id,
            data: vec![0u8; size],
            priority,
            created_at: Instant::now(),
        }
    }

    fn process(&mut self) -> u64 {
        // Simulate some work
        let mut hash: u64 = self.id;
        for byte in &self.data {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        hash
    }
}

/// Work queue for stress testing
#[allow(dead_code)]
struct WorkQueue {
    pending: VecDeque<WorkUnit>,
    processed: u64,
    total_wait_time: Duration,
    max_queue_depth: usize,
}

#[allow(dead_code)]
impl WorkQueue {
    fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            processed: 0,
            total_wait_time: Duration::ZERO,
            max_queue_depth: 0,
        }
    }

    fn enqueue(&mut self, work: WorkUnit) {
        self.pending.push_back(work);
        self.max_queue_depth = self.max_queue_depth.max(self.pending.len());
    }

    fn dequeue(&mut self) -> Option<WorkUnit> {
        self.pending.pop_front().inspect(|work| {
            self.processed += 1;
            self.total_wait_time += work.created_at.elapsed();
        })
    }

    fn average_wait_time(&self) -> Duration {
        if self.processed == 0 {
            Duration::ZERO
        } else {
            self.total_wait_time / self.processed as u32
        }
    }
}

/// Contention simulator
#[allow(dead_code)]
struct ContentionSimulator {
    counter: Arc<AtomicU64>,
    local_work: Vec<u64>,
}

#[allow(dead_code)]
impl ContentionSimulator {
    fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
            local_work: Vec::with_capacity(1000),
        }
    }

    fn do_contended_work(&self, iterations: usize) -> u64 {
        let mut sum = 0u64;
        for _ in 0..iterations {
            // Atomic increment simulating contention
            let val = self.counter.fetch_add(1, Ordering::SeqCst);
            sum = sum.wrapping_add(val);
        }
        sum
    }

    fn do_local_work(&mut self, iterations: usize) -> u64 {
        self.local_work.clear();
        for i in 0..iterations {
            self.local_work.push(i as u64 * 7);
        }
        self.local_work.iter().sum()
    }

    fn get_counter(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
}

/// Cache thrashing simulator
struct CacheTrasher {
    data: Vec<Vec<u64>>,
    access_pattern: Vec<usize>,
}

impl CacheTrasher {
    fn new(arrays: usize, array_size: usize) -> Self {
        let mut gen = StressPatternGenerator::new(42);
        let data: Vec<Vec<u64>> = (0..arrays)
            .map(|_| (0..array_size).map(|i| i as u64).collect())
            .collect();

        // Random access pattern to defeat prefetcher
        let access_pattern: Vec<usize> = (0..1000).map(|_| gen.next_usize(arrays)).collect();

        Self {
            data,
            access_pattern,
        }
    }

    fn sequential_access(&self) -> u64 {
        let mut sum = 0u64;
        for array in &self.data {
            for &val in array {
                sum = sum.wrapping_add(val);
            }
        }
        sum
    }

    fn random_access(&self) -> u64 {
        let mut sum = 0u64;
        for &idx in &self.access_pattern {
            if idx < self.data.len() {
                for &val in &self.data[idx] {
                    sum = sum.wrapping_add(val);
                }
            }
        }
        sum
    }

    fn strided_access(&self, stride: usize) -> u64 {
        let mut sum = 0u64;
        for array in &self.data {
            let mut i = 0;
            while i < array.len() {
                sum = sum.wrapping_add(array[i]);
                i += stride;
            }
        }
        sum
    }
}

/// GC pressure simulator (simulates allocation churn)
struct GcPressureSimulator {
    generations: Vec<VecDeque<Vec<u8>>>,
    gen0_limit: usize,
    gen1_limit: usize,
}

impl GcPressureSimulator {
    fn new() -> Self {
        Self {
            generations: vec![VecDeque::new(), VecDeque::new(), VecDeque::new()],
            gen0_limit: 100,
            gen1_limit: 50,
        }
    }

    fn allocate(&mut self, size: usize) {
        // Gen 0 allocation
        self.generations[0].push_back(vec![0u8; size]);

        // Simulate promotion
        if self.generations[0].len() > self.gen0_limit {
            // Promote some to Gen 1
            for _ in 0..10 {
                if let Some(item) = self.generations[0].pop_front() {
                    self.generations[1].push_back(item);
                }
            }
        }

        if self.generations[1].len() > self.gen1_limit {
            // Promote some to Gen 2
            for _ in 0..5 {
                if let Some(item) = self.generations[1].pop_front() {
                    self.generations[2].push_back(item);
                }
            }
        }

        // Gen 2 cleanup (simulated full GC)
        if self.generations[2].len() > 100 {
            self.generations[2].truncate(50);
        }
    }

    fn total_allocated(&self) -> usize {
        self.generations
            .iter()
            .map(|gen| gen.iter().map(|v| v.len()).sum::<usize>())
            .sum()
    }
}

// ============================================================================
// BENCHMARK GROUPS
// ============================================================================

fn bench_measurement_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("measurement_overhead");

    // Baseline: measure nothing
    group.bench_function("baseline_empty", |b| b.iter(|| std_black_box(())));

    // Instant::now overhead
    group.bench_function("instant_now_overhead", |b| {
        b.iter(|| std_black_box(Instant::now()))
    });

    // Duration calculation overhead
    group.bench_function("duration_calculation", |b| {
        let start = Instant::now();
        b.iter(|| {
            let elapsed = start.elapsed();
            std_black_box(elapsed)
        })
    });

    // black_box overhead
    group.bench_function("black_box_overhead_u64", |b| {
        let value = 42u64;
        b.iter(|| std_black_box(value))
    });

    // Statistical sampling overhead
    group.bench_function("stats_collection_1000", |b| {
        b.iter(|| {
            let mut stats = BenchmarkStats::new();
            for i in 0..1000 {
                stats.add_sample(i as f64);
            }
            std_black_box((stats.mean(), stats.std_dev()))
        })
    });

    group.finish();
}

fn bench_statistical_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistical_stability");

    // Coefficient of variation should be low for stable benchmarks
    for iterations in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("cv_analysis", iterations),
            &iterations,
            |b, &iterations| {
                b.iter(|| {
                    let mut stats = BenchmarkStats::new();
                    let mut gen = StressPatternGenerator::new(12345);

                    for _ in 0..iterations {
                        // Simulate timing samples with some noise
                        let base = 1000.0; // 1000ns base
                        let noise = gen.next_f64() * 50.0; // ±25ns noise
                        stats.add_sample(base + noise - 25.0);
                    }

                    let cv = stats.coefficient_of_variation();
                    std_black_box(cv)
                })
            },
        );
    }

    // Percentile stability
    group.bench_function("percentile_stability_10000", |b| {
        b.iter(|| {
            let mut stats = BenchmarkStats::new();
            let mut gen = StressPatternGenerator::new(54321);

            for _ in 0..10000 {
                // Log-normal-ish distribution (common for latencies)
                let base = gen.next_f64().ln().abs() * 100.0 + 100.0;
                stats.add_sample(base);
            }

            let p50 = stats.percentile(50.0);
            let p90 = stats.percentile(90.0);
            let p99 = stats.percentile(99.0);
            std_black_box((p50, p90, p99))
        })
    });

    group.finish();
}

fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Allocation burst pattern
    for burst_size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(burst_size as u64));

        group.bench_with_input(
            BenchmarkId::new("allocation_burst", burst_size),
            &burst_size,
            |b, &size| {
                b.iter(|| {
                    let allocations: Vec<Vec<u8>> = (0..size).map(|_| vec![0u8; 1024]).collect();
                    std_black_box(allocations.len())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("allocation_tracked", burst_size),
            &burst_size,
            |b, &size| {
                b.iter(|| {
                    let mut tracker = AllocationTracker::new();
                    let mut allocations = Vec::new();

                    for _ in 0..size {
                        let alloc = vec![0u8; 1024];
                        tracker.allocate(alloc.len());
                        allocations.push(alloc);
                    }

                    std_black_box((tracker.peak_allocated, tracker.total_allocated))
                })
            },
        );
    }

    // GC pressure simulation
    group.bench_function("gc_pressure_churn_1000", |b| {
        b.iter(|| {
            let mut simulator = GcPressureSimulator::new();

            for _ in 0..1000 {
                simulator.allocate(256);
            }

            std_black_box(simulator.total_allocated())
        })
    });

    group.finish();
}

fn bench_work_queue_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("work_queue_stress");

    for queue_depth in [10, 100, 1000] {
        group.throughput(Throughput::Elements(queue_depth as u64));

        group.bench_with_input(
            BenchmarkId::new("enqueue_dequeue", queue_depth),
            &queue_depth,
            |b, &depth| {
                b.iter(|| {
                    let mut queue = WorkQueue::new();

                    // Fill queue
                    for i in 0..depth {
                        queue.enqueue(WorkUnit::new(i as u64, 64, i as u32 % 10));
                    }

                    // Process all
                    let mut results = Vec::with_capacity(depth);
                    while let Some(mut work) = queue.dequeue() {
                        results.push(work.process());
                    }

                    std_black_box((queue.processed, results.len()))
                })
            },
        );

        // Interleaved enqueue/dequeue
        group.bench_with_input(
            BenchmarkId::new("interleaved_ops", queue_depth),
            &queue_depth,
            |b, &depth| {
                b.iter(|| {
                    let mut queue = WorkQueue::new();
                    let mut gen = StressPatternGenerator::new(42);
                    let mut processed = 0u64;

                    for i in 0..(depth * 2) {
                        // 70% chance to enqueue, 30% to dequeue
                        if gen.next_f64() < 0.7 || queue.pending.is_empty() {
                            queue.enqueue(WorkUnit::new(i as u64, 64, 0));
                        } else if let Some(mut work) = queue.dequeue() {
                            processed += work.process();
                        }
                    }

                    // Drain remaining
                    while let Some(mut work) = queue.dequeue() {
                        processed += work.process();
                    }

                    std_black_box(processed)
                })
            },
        );
    }

    group.finish();
}

fn bench_cache_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_effects");

    // L1 cache size test (32KB typical)
    // L2 cache size test (256KB typical)
    // L3 cache size test (8MB+ typical)
    let sizes = [
        (8, "8KB_L1"),
        (32, "32KB_L1"),
        (256, "256KB_L2"),
        (1024, "1MB_L2L3"),
        (8192, "8MB_L3"),
    ];

    for (size_kb, name) in sizes {
        let elements = (size_kb * 1024) / 8; // u64 = 8 bytes

        group.throughput(Throughput::Bytes((size_kb * 1024) as u64));

        group.bench_with_input(
            BenchmarkId::new("sequential", name),
            &elements,
            |b, &count| {
                let data: Vec<u64> = (0..count).map(|i| i as u64).collect();
                b.iter(|| {
                    let sum: u64 = data.iter().sum();
                    std_black_box(sum)
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("random", name), &elements, |b, &count| {
            let data: Vec<u64> = (0..count).map(|i| i as u64).collect();
            let mut gen = StressPatternGenerator::new(42);
            let indices: Vec<usize> = (0..count).map(|_| gen.next_usize(count)).collect();

            b.iter(|| {
                let mut sum = 0u64;
                for &idx in &indices {
                    sum = sum.wrapping_add(data[idx]);
                }
                std_black_box(sum)
            })
        });
    }

    // Cache line bouncing
    group.bench_function("cache_line_bouncing", |b| {
        let trasher = CacheTrasher::new(64, 1024);

        b.iter(|| {
            let seq = trasher.sequential_access();
            let rand = trasher.random_access();
            std_black_box((seq, rand))
        })
    });

    // Stride effects
    for stride in [1, 2, 4, 8, 16, 64] {
        group.bench_with_input(
            BenchmarkId::new("stride_access", stride),
            &stride,
            |b, &stride| {
                let trasher = CacheTrasher::new(16, 4096);
                b.iter(|| std_black_box(trasher.strided_access(stride)))
            },
        );
    }

    group.finish();
}

fn bench_contention_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention_scenarios");

    // Atomic contention simulation
    for iterations in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("atomic_increment", iterations),
            &iterations,
            |b, &iterations| {
                let simulator = ContentionSimulator::new();
                b.iter(|| std_black_box(simulator.do_contended_work(iterations)))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("local_work", iterations),
            &iterations,
            |b, &iterations| {
                let mut simulator = ContentionSimulator::new();
                b.iter(|| std_black_box(simulator.do_local_work(iterations)))
            },
        );
    }

    // Contention vs local work ratio
    group.bench_function("contention_ratio_analysis", |b| {
        let mut simulator = ContentionSimulator::new();

        b.iter(|| {
            // Mix of contended and local work
            let contended = simulator.do_contended_work(100);
            let local = simulator.do_local_work(1000);
            std_black_box((contended, local))
        })
    });

    group.finish();
}

fn bench_stress_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_patterns");

    // Burst pattern stress
    group.bench_function("burst_pattern_1000", |b| {
        let mut gen = StressPatternGenerator::new(12345);

        b.iter(|| {
            let mut total_work = 0u64;

            for i in 0..1000 {
                if gen.burst_pattern(i) {
                    // Burst phase: heavy work
                    for j in 0..100 {
                        total_work += j as u64 * gen.next_u64();
                    }
                } else {
                    // Calm phase: light work
                    total_work += gen.next_u64();
                }
            }

            std_black_box(total_work)
        })
    });

    // Random walk stress
    group.bench_function("random_walk_1000", |b| {
        let mut gen = StressPatternGenerator::new(54321);

        b.iter(|| {
            let mut value = 100.0;
            let mut history = Vec::with_capacity(1000);

            for _ in 0..1000 {
                value = gen.random_walk(value, 10.0);
                history.push(value);
            }

            let avg: f64 = history.iter().sum::<f64>() / history.len() as f64;
            std_black_box(avg)
        })
    });

    // Combined stress
    group.bench_function("combined_stress_pattern", |b| {
        let mut gen = StressPatternGenerator::new(98765);

        b.iter(|| {
            let mut queue = WorkQueue::new();
            let mut allocator = AllocationTracker::new();
            let mut value = 100.0;

            for i in 0..500 {
                value = gen.random_walk(value, 5.0);
                let size = (value as usize).clamp(64, 4096);

                if gen.burst_pattern(i) {
                    // Burst: multiple allocations and queue operations
                    for j in 0..10 {
                        allocator.allocate(size);
                        queue.enqueue(WorkUnit::new(i as u64 * 10 + j, size, j as u32));
                    }
                } else {
                    // Normal: single operation
                    allocator.allocate(size);
                    queue.enqueue(WorkUnit::new(i as u64, size, 0));
                }

                // Process some work
                if let Some(mut work) = queue.dequeue() {
                    allocator.deallocate(work.data.len());
                    let _ = work.process();
                }
            }

            std_black_box((queue.processed, allocator.total_allocated))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_measurement_overhead,
    bench_statistical_stability,
    bench_memory_patterns,
    bench_work_queue_stress,
    bench_cache_effects,
    bench_contention_scenarios,
    bench_stress_patterns,
);

criterion_main!(benches);
