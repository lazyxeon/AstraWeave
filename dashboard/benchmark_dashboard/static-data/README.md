# Static Benchmark Snapshot

The files in this directory are checked into the repository so the dashboard ships with meaningful data when served from GitHub Pages or any static host.

- `history.jsonl` — condensed benchmark history that matches the JSON Lines schema exported by `scripts/export_benchmark_jsonl.ps1`.
- `metadata.json` — describes when the snapshot was generated and the time span it covers.

During CI (or whenever you refresh benchmarks locally), run:

```powershell
.\scripts




That script invokes `export_benchmark_jsonl.ps1`, copies the latest artifacts into `tools/benchmark-dashboard/static-data/`, and prepares the dashboard for publication.```un_benchmark_dashboard.ps1 -UpdateStaticSnapshot