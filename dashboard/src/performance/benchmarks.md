# Benchmarks

This page is how to run Criterion in this repository and where the numbers are kept. It does not paste a result table. Dated figures, including the GPU frame sample, are on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page.

## Run one bench

From the repository root, with the pinned toolchain (`rustc --version` is 1.89.0 here):

```bash
cargo bench -p astraweave-ecs --bench ecs_benchmarks
cargo bench -p astraweave-ai --bench ai_core_loop
cargo bench -p astraweave-behavior --bench goap_planning
cargo bench -p astraweave-behavior --bench behavior_tree
cargo bench -p astraweave-terrain --bench terrain_generation
cargo bench -p astraweave-input --bench input_benchmarks
cargo bench -p astraweave-physics --bench raycast
cargo bench -p astraweave-physics --bench character_controller
cargo bench -p astraweave-physics --bench rigid_body
cargo bench -p astraweave-core --bench core_benchmarks
cargo bench -p astraweave-core --bench full_game_loop
```

Each `name` is a `[[bench]]` entry in that crate's `Cargo.toml`. `harness = false` means the bench file drives Criterion itself.

`ecs_benchmarks` is on **astraweave-ecs**, not astraweave-core. `.github/workflows/benchmark-dashboard.yml` still runs `cargo bench -p astraweave-core --bench ecs_benchmarks`. That package does not define that bench name. Use the `astraweave-ecs` command above. The workflow marks the bench step `continue-on-error: true`, so a miss there does not fail the job.

Criterion writes under `target/criterion/` for the package you benched (reports and sample data for that run). A number from that directory is your machine and your commit. Copy the commit (`git rev-parse HEAD`) next to it. Do not paste it back onto this page as the project's latest run.

`cargo bench -p <crate>` with no `--bench` runs every bench target that crate declares. That is the long run.

## The published dashboard

Workflow `.github/workflows/benchmark-dashboard.yml` (schedule `0 2 * * *`, `workflow_dispatch`, and pushes that touch that workflow) runs the subset listed in its "Run Benchmarks" step, then `pwsh -File scripts/export_benchmark_jsonl.ps1`, and publishes to `dashboard/benchmark_dashboard`.

The page that job points at is:

https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/

The historical write-up in git is [MASTER_BENCHMARK_REPORT.md](https://github.com/lazyxeon/AstraWeave/blob/main/docs/masters/MASTER_BENCHMARK_REPORT.md). Engine status says that report is not a fresh full-frame budget.

## The frame sample is not a Criterion row

The GPU frame budget is a separate capture, `gpu_frame_budget`, documented on the engine status page with the adapter and the commit. A cloud shell cannot measure the GTX 1660 Ti named there. Lavapipe time is not that budget. Do not substitute a CPU bench for it.

## What this page used to show

A January 2026 grid (picosecond rows, a 1.27 ms frame, an arbiter row, a 99.96% spatial-hash cell) was presented as current, with a "Production Ready" badge. Those figures are removed. `PhysicsWorld` uses Rapier's `DefaultBroadPhase`. `AIArbiter` is not on the sim tick. Re-measure with the commands above if you need a number.

## Related

- [Methodology](./methodology.md)
- [Budgets](./budgets.md)
- [Optimization](./optimization.md)
- [Engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md)
