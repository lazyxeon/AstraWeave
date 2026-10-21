"""
Render crate full-sweep driver: 20 shards, sequential execution.
Usage: python scripts/run_render_sweep.py [start_shard]
Saves aggregate.json and summary.txt after each shard.
"""
import json, os, shutil, subprocess, sys, time
from pathlib import Path

CRATE   = "astraweave-render"
SHARDS  = 20
TIMEOUT = 300
JOBS    = 1
TMP     = Path(r"C:\temp")
OUTDIR  = TMP / "mutants.out"
RESULTS = Path("render_sweep_results")
RESULTS.mkdir(exist_ok=True)

# Extra features to maximize mutant coverage
EXTRA_FEATURES = "textures,nanite,ssao,bloom,skinning-gpu"

def cleanup_temp():
    if OUTDIR.exists():
        shutil.rmtree(OUTDIR, ignore_errors=True)

def run_shard(shard: int):
    cmd = [
        "cargo", "mutants",
        "-p", CRATE,
        "--shard", f"{shard}/{SHARDS}",
        "--timeout", str(TIMEOUT),
        "-j", str(JOBS),
        "-o", str(TMP),
        "--features", EXTRA_FEATURES,
        "--", "--lib",
    ]
    print(f"\n{'='*60}")
    print(f"[SHARD {shard}/{SHARDS}] Starting at {time.strftime('%H:%M:%S')}")
    print(f"{'='*60}")
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=7200)
    print(f"[SHARD {shard}/{SHARDS}] Completed with code {result.returncode}")
    return result.returncode

def collect_outcomes(shard: int) -> dict:
    outcomes_path = OUTDIR / "outcomes.json"
    if not outcomes_path.exists():
        return {"shard": shard, "outcomes": [], "error": "no outcomes.json"}
    with open(outcomes_path) as f:
        return json.load(f)

def main():
    start = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    aggregate = {"shards": {}, "summary": {"caught": 0, "missed": 0, "unviable": 0, "timeout": 0}}

    # Load existing if resuming
    agg_path = RESULTS / "aggregate.json"
    if agg_path.exists() and start > 0:
        with open(agg_path) as f:
            aggregate = json.load(f)

    for shard in range(start, SHARDS):
        cleanup_temp()
        rc = run_shard(shard)
        data = collect_outcomes(shard)
        aggregate["shards"][str(shard)] = data

        # Tally
        for o in data.get("outcomes", []):
            s = o.get("summary", "")
            if "Caught" in s: aggregate["summary"]["caught"] += 1
            elif "Missed" in s: aggregate["summary"]["missed"] += 1
            elif "Unviable" in s: aggregate["summary"]["unviable"] += 1
            elif "Timeout" in s: aggregate["summary"]["timeout"] += 1

        # Save progress
        with open(agg_path, "w") as f:
            json.dump(aggregate, f, indent=2)

        total = aggregate["summary"]
        viable = total["caught"] + total["missed"]
        rate = (total["caught"] / viable * 100) if viable else 0
        summary = (
            f"After shard {shard}: "
            f"caught={total['caught']} missed={total['missed']} "
            f"unviable={total['unviable']} timeout={total['timeout']} "
            f"kill_rate={rate:.1f}%"
        )
        print(summary)
        with open(RESULTS / "summary.txt", "w") as f:
            f.write(summary + "\n")

    print("\n=== RENDER SWEEP COMPLETE ===")
    print(json.dumps(aggregate["summary"], indent=2))

if __name__ == "__main__":
    main()
