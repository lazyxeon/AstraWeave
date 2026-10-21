# Benchmark Dashboard Static Snapshot Plan

This document captures the agreed workflow for bundling pre-generated benchmark data with the dashboard so it renders instantly when hosted on GitHub Pages or an internal file server.

## Goals

1. **Ship baked benchmarks** – Checked-in JSONL + metadata assets under `tools/benchmark-dashboard/static-data/` load immediately without requiring contributors to run benchmarks first.
2. **Keep live workflows intact** – A dedicated UI action switches the dashboard back to live data collected on the developer machine (`target/benchmark-data/`).
3. **Provide self-serve instructions** – A "Run on your device" panel surfaces the exact PowerShell commands needed to re-export benchmarks and refresh the static snapshot.

## Directory Layout

```
tools/benchmark-dashboard/
├── static-data/
│   ├── history.jsonl      # Baked benchmark timeline (≤ 60 days)
│   └── metadata.json      # Snapshot metadata: generated_at, commit hash, record counts
├── benchmark-data/        # (optional) staging folder populated by run scripts
├── index.html             # Updated UI with Run Locally button + panel
└── dashboard.js           # Loader swaps between static and live data sources
```

## Data Flow

1. `cargo bench` + `scripts/export_benchmark_jsonl.ps1` still writes to `target/benchmark-data/`.
2. The export script now mirrors both `history.jsonl` and `metadata.json` into `tools/benchmark-dashboard/static-data/` so Pages always has fresh data.
3. `scripts/run_benchmark_dashboard.ps1` copies the latest artifacts into `tools/benchmark-dashboard/benchmark-data/` (for live reload testing) and reports the location of the baked snapshot.

## Front-End Behavior

- **Default load path**: `dashboard.js` first tries `static-data/history.jsonl`. If successful, the header badge indicates "Static Snapshot" with the metadata timestamp.
- **Run Locally button**: Attempts to load from developer-owned sources (`benchmark-data/history.jsonl`, `../../target/benchmark-data/history.jsonl`, etc.). On success, the badge switches to "Live Benchmarks".
- **Failure handling**: When a live reload fails, the dashboard keeps the static dataset and displays a toast/inline notice describing the error.
- **Run-on-your-device panel**: Presents the canonical PowerShell commands with copy buttons:
  - `cargo bench -p <targets>` followed by `scripts/export_benchmark_jsonl.ps1`
  - `scripts/run_benchmark_dashboard.ps1 -UpdateStaticSnapshot`

## Documentation Touchpoints

- **Root README**: Dashboard button links to `/tools/benchmark-dashboard/`, explains that the page now ships with a baked dataset, and references the Run Locally workflow.
- **tools/benchmark-dashboard/README**: Adds a "Static Snapshot" section covering snapshot maintenance, CI expectations, and how to refresh the bundled assets.

## CI/Automation

- Extend the benchmark CI workflow (or nightly job) to call `scripts/export_benchmark_jsonl.ps1 -SyncStaticData`. This ensures `static-data/` is always up to date before publishing to GitHub Pages.
- Optionally add a helper script to validate that `static-data/history.jsonl` is newer than 48 hours; CI can fail if the snapshot grows stale.

This plan must be implemented before the dashboard redesign ships so contributors always see meaningful data without manual setup.
