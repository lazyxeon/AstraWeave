//! Benchmarks for AIArbiter performance validation
//!
//! Tests the GOAP+Hermes Hybrid Arbiter pattern with focus on:
//! - GOAP mode update latency (<100 µs target)
//! - ExecutingLLM mode update latency (<50 µs target)
//! - Mode transition overhead (<10 µs target)
//! - LLM poll overhead (<10 µs target)
//!
//! These benchmarks validate the "zero user-facing latency" promise:
//! - GOAP returns instantly while LLM runs in background
//! - Plan execution is O(1) array access
//! - Mode transitions are minimal overhead
//! - Non-blocking polling adds <10 µs overhead

#[cfg(feature = "llm_orchestrator")]
use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(feature = "llm_orchestrator")]
use std::hint::black_box;

#[cfg(feature = "llm_orchestrator")]
mod benchmarks {
    use super::*;
    use astraweave_ai::{AIArbiter, LlmExecutor, Orchestrator, OrchestratorAsync};
    use astraweave_core::{
        ActionStep, CompanionState, EnemyState, IVec2, PlanIntent, PlayerState, WorldSnapshot,
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;

    // ========================================================================
    // Benchmark Helpers
    // ========================================================================

    /// Create a test world snapshot
    fn create_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "standing".into(),
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2 { x: 0, y: 0 },
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
            },
            enemies: vec![EnemyState {
                id: 1, // Entity is just u32
                pos: IVec2 { x: 5, y: 0 },
                hp: 50,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("Test objective".into()),

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    /// Create a mock plan with N steps
    fn create_mock_plan(steps: usize) -> PlanIntent {
        PlanIntent {
            plan_id: format!("bench-plan-{}", steps),
            steps: (0..steps)
                .map(|i| ActionStep::MoveTo {
                    x: i as i32,
                    y: 0,
                    speed: None,
                })
                .collect(),
        }
    }

    // ========================================================================
    // Mock Orchestrators (Minimal Overhead)
    // ========================================================================

    struct BenchGoap;

    impl Orchestrator for BenchGoap {
        fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
            PlanIntent {
                plan_id: "bench-goap".into(),
                steps: vec![ActionStep::MoveTo {
                    x: 5,
                    y: 5,
                    speed: None,
                }],
            }
        }
    }

    struct BenchBT;

    impl Orchestrator for BenchBT {
        fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
            PlanIntent {
                plan_id: "bench-bt".into(),
                steps: vec![ActionStep::Wait { duration: 1.0 }],
            }
        }
    }

    /// Dummy async orchestrator (never actually called in benchmarks)
    struct BenchLlmOrch;

    #[async_trait::async_trait]
    impl OrchestratorAsync for BenchLlmOrch {
        async fn plan(&self, _snap: WorldSnapshot, _budget_ms: u32) -> anyhow::Result<PlanIntent> {
            // Never called in sync benchmarks
            Ok(create_mock_plan(1))
        }

        fn name(&self) -> &'static str {
            "BenchLlmOrch"
        }
    }

    /// Create a minimal arbiter for benchmarking
    fn create_arbiter() -> AIArbiter {
        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(Arc::new(BenchLlmOrch), runtime);
        let goap = Box::new(BenchGoap);
        let bt = Box::new(BenchBT);

        AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(999999.0)
        // Prevent LLM requests
    }

    /// Create arbiter in ExecutingLLM mode with a plan
    fn create_arbiter_executing_llm(plan_size: usize) -> AIArbiter {
        let mut arbiter = create_arbiter();
        arbiter.transition_to_llm(create_mock_plan(plan_size));
        arbiter
    }

    // ========================================================================
    // Benchmark 1: GOAP Mode Update
    // ========================================================================

    pub fn bench_arbiter_goap_update(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        c.bench_function("arbiter_goap_update", |b| {
            let mut arbiter = create_arbiter();
            let snap = create_test_snapshot();

            b.iter(|| {
                let action = arbiter.update(black_box(&snap));
                black_box(action);
            });
        });
    }

    // ========================================================================
    // Benchmark 2: ExecutingLLM Mode Update
    // ========================================================================

    pub fn bench_arbiter_executing_llm_update(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        c.bench_function("arbiter_executing_llm_update", |b| {
            let snap = create_test_snapshot();

            b.iter_batched(
                || create_arbiter_executing_llm(100), // Large plan to avoid exhaustion
                |mut arbiter| {
                    let action = arbiter.update(black_box(&snap));
                    black_box(action);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    // ========================================================================
    // Benchmark 3: Mode Transitions (Simplified - only transition_to_llm)
    // ========================================================================

    pub fn bench_arbiter_mode_transitions(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        c.bench_function("arbiter_mode_transition_to_llm", |b| {
            b.iter_batched(
                create_arbiter,
                |mut arbiter| {
                    // Benchmark transition to LLM mode (GOAP → ExecutingLLM)
                    arbiter.transition_to_llm(black_box(create_mock_plan(1)));
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    // ========================================================================
    // Benchmark 4: LLM Poll Overhead (No Active Task)
    // ========================================================================

    pub fn bench_arbiter_llm_poll_no_task(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        c.bench_function("arbiter_llm_poll_no_task", |b| {
            let mut arbiter = create_arbiter();
            let snap = create_test_snapshot();

            b.iter(|| {
                // Update in GOAP mode with no active LLM task
                // This benchmarks the poll_llm_result() overhead when there's nothing to poll
                let action = arbiter.update(black_box(&snap));
                black_box(action);
            });
        });
    }

    // ========================================================================
    // Benchmark 5: Full Update Cycle (GOAP → LLM → ExecutingLLM → GOAP)
    // ========================================================================

    pub fn bench_arbiter_full_cycle(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        c.bench_function("arbiter_full_cycle", |b| {
            let snap = create_test_snapshot();

            b.iter_batched(
                || {
                    let mut arbiter = create_arbiter();
                    // Manually inject plan to simulate LLM completion
                    arbiter.transition_to_llm(create_mock_plan(3));
                    arbiter
                },
                |mut arbiter| {
                    // Execute 3-step plan + auto-transition back to GOAP
                    let action1 = arbiter.update(black_box(&snap));
                    let action2 = arbiter.update(black_box(&snap));
                    let action3 = arbiter.update(black_box(&snap));
                    let action4 = arbiter.update(black_box(&snap)); // Back in GOAP
                    black_box((action1, action2, action3, action4));
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    criterion_group!(
        arbiter_benches,
        bench_arbiter_goap_update,
        bench_arbiter_executing_llm_update,
        bench_arbiter_mode_transitions,
        bench_arbiter_llm_poll_no_task,
        bench_arbiter_full_cycle,
    );
}

#[cfg(feature = "llm_orchestrator")]
criterion_main!(benchmarks::arbiter_benches);

#[cfg(not(feature = "llm_orchestrator"))]
fn main() {
    println!("Arbiter benchmarks require llm_orchestrator feature");
}
