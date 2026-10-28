# Benchmarks

Measured figures for this tree are on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page. That page names the commit, the adapter, and which numbers were not re-run.

The historical Criterion write-up is [MASTER_BENCHMARK_REPORT.md](https://github.com/lazyxeon/AstraWeave/blob/main/docs/masters/MASTER_BENCHMARK_REPORT.md). It is not a fresh capture, and it is not a passing full-frame budget.

This page used to present a January 2026 dashboard (picosecond operation times, a 1.27 ms frame, an arbiter row at 101.7 ns, and a spatial-hash reduction of 99.96%) as current results, with a "Production Ready (Grade A+)" badge. Those figures are removed. The 99.96% line is a comment on `SpatialHash`. `PhysicsWorld` uses Rapier's `DefaultBroadPhase`. `AIArbiter` is not on the sim tick, and its nanosecond table is not on the engine status page.

Reproduce a crate bench with `cargo bench -p <crate>` on a machine you control. Record the commit and the CPU or GPU. Do not paste an undated number back onto this page.

The GPU frame budget and the GTX 1660 Ti sample are on the engine status page. A cloud agent cannot measure that card. Lavapipe time is not that budget.

## Related

- [Methodology](./methodology.md)
- [Budgets](./budgets.md)
- [Optimization](./optimization.md)
