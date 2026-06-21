# AstraWeave: Master Benchmark Report (Current)

The authoritative benchmark report currently lives in:

- [docs/masters/MASTER_BENCHMARK_REPORT.md](../masters/MASTER_BENCHMARK_REPORT.md) (v5.59, 2026-06-19)

This stub exists to keep master-report navigation consistent under `docs/current/`.

> **⚠ W.1 note (W-series, 2026-06-21):** the v5.59 voxel-sparsity result summarized
> below benchmarks the `WaterVolumeGrid` voxel sim **removed in W.1** (commit
> `1a57fdd41`; recovery tag `w0-pre-deprecation`) — it is the historical evidence
> behind the decline-A→C verdict, not current code. The canonical report at
> `docs/masters/MASTER_BENCHMARK_REPORT.md` carries the same annotation in full.

**Latest update**: v5.59 — Fluids-Integration F.3.S voxel sparsity vs dense (same min-spec box): dirty-AABB `simulate` is bit-identical to dense `simulate_reference` and up to 14× faster for localized water, but the ~1 ms budget at 64³ is met ONLY for small grids (32³ ≤50 %) or localized water ≲16³ — a full-extent 64³ flood is 2.35 ms even at 5 %. Verdict PARTIAL → stay at Option A (column-coupled pressure + the F.3 forward-cascade are the walls). Prior: v5.58 — first fluids baselines (F.1).
