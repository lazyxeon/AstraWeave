#!/usr/bin/env python3
"""
Terrain Mutation Testing Full Sweep Driver
Runs all 22 shards of astraweave-terrain mutation testing sequentially,
saving per-shard summaries and aggregated outcomes.
"""
import subprocess
import json
import os
import shutil
import time
import sys
from pathlib import Path
from collections import Counter
from datetime import datetime

CRATE = "astraweave-terrain"
TOTAL_SHARDS = 22
TIMEOUT = 300
JOBS = 1
TEMP_DIR = r"C:\temp"
OUTCOMES_PATH = os.path.join(TEMP_DIR, "mutants.out", "outcomes.json")
WORKSPACE = r"C:\Users\pv2br\AstraWeave-AI-Native-Gaming-Engine"
RESULTS_DIR = os.path.join(WORKSPACE, "terrain_sweep_results")
AGGREGATE_FILE = os.path.join(RESULTS_DIR, "aggregate.json")
SUMMARY_FILE = os.path.join(RESULTS_DIR, "summary.txt")

def cleanup_temp():
    """Remove temp directories from previous runs."""
    # Clean cargo-mutants dirs in TEMP
    temp_env = os.environ.get("TEMP", "")
    if temp_env and os.path.exists(temp_env):
        for d in os.listdir(temp_env):
            if d.startswith("cargo-mutants"):
                full = os.path.join(temp_env, d)
                try:
                    shutil.rmtree(full)
                except Exception:
                    pass
    # Clean C:\temp
    if os.path.exists(TEMP_DIR):
        try:
            shutil.rmtree(TEMP_DIR)
        except Exception:
            pass

def run_shard(shard_num):
    """Run a single shard and return the outcomes."""
    cleanup_temp()
    
    cmd = [
        "cargo", "mutants",
        "-p", CRATE,
        "--shard", f"{shard_num}/{TOTAL_SHARDS}",
        "--timeout", str(TIMEOUT),
        "-j", str(JOBS),
        "-o", TEMP_DIR,
        "--", "--lib"
    ]
    
    print(f"\n{'='*60}")
    print(f"SHARD {shard_num}/{TOTAL_SHARDS} - Starting at {datetime.now().strftime('%H:%M:%S')}")
    print(f"{'='*60}")
    sys.stdout.flush()
    
    start = time.time()
    result = subprocess.run(
        cmd,
        cwd=WORKSPACE,
        capture_output=True,
        text=True
    )
    elapsed = time.time() - start
    
    # Extract summary from stderr/stdout
    output = result.stdout + "\n" + result.stderr
    summary_line = ""
    for line in output.split("\n"):
        if "mutants tested" in line:
            summary_line = line.strip()
            break
    
    print(f"  Elapsed: {elapsed:.0f}s ({elapsed/60:.1f}m)")
    print(f"  Summary: {summary_line}")
    sys.stdout.flush()
    
    # Parse outcomes.json
    outcomes = []
    shard_stats = {"shard": shard_num, "elapsed_s": elapsed, "summary": summary_line}
    
    if os.path.exists(OUTCOMES_PATH):
        with open(OUTCOMES_PATH) as f:
            data = json.load(f)
        
        outcomes = data.get("outcomes", [])
        shard_stats["total"] = data.get("total_mutants", 0)
        shard_stats["caught"] = data.get("caught", 0)
        shard_stats["missed"] = data.get("missed", 0)
        shard_stats["unviable"] = data.get("unviable", 0)
        shard_stats["timeout"] = data.get("timeout", 0)
        
        # Per-file breakdown
        miss_files = Counter(
            x["scenario"]["Mutant"]["file"]
            for x in outcomes if x["summary"] == "MissedMutant"
        )
        caught_files = Counter(
            x["scenario"]["Mutant"]["file"]
            for x in outcomes if x["summary"] == "CaughtMutant"
        )
        shard_stats["miss_by_file"] = dict(miss_files.most_common())
        shard_stats["caught_by_file"] = dict(caught_files.most_common())
        
        print(f"  Caught: {shard_stats['caught']}, Missed: {shard_stats['missed']}, Unviable: {shard_stats['unviable']}")
        if miss_files:
            print(f"  Top misses: {', '.join(f'{k.split('/')[-1]}({v})' for k,v in miss_files.most_common(3))}")
        sys.stdout.flush()
    else:
        print("  WARNING: No outcomes.json found!")
        shard_stats["error"] = "no outcomes.json"
    
    return shard_stats, outcomes

def main():
    start_shard = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    
    os.makedirs(RESULTS_DIR, exist_ok=True)
    
    # Load existing aggregate if resuming
    all_stats = []
    all_outcomes = []
    if os.path.exists(AGGREGATE_FILE) and start_shard > 0:
        with open(AGGREGATE_FILE) as f:
            prev = json.load(f)
        all_stats = prev.get("shard_stats", [])
        all_outcomes = prev.get("all_outcomes", [])
        print(f"Resuming from shard {start_shard} (loaded {len(all_stats)} previous shards)")
    
    total_start = time.time()
    
    for shard in range(start_shard, TOTAL_SHARDS):
        stats, outcomes = run_shard(shard)
        all_stats.append(stats)
        all_outcomes.extend(outcomes)
        
        # Save progress after each shard
        aggregate = {
            "crate": CRATE,
            "total_shards": TOTAL_SHARDS,
            "completed_shards": len(all_stats),
            "last_shard": shard,
            "shard_stats": all_stats,
            "all_outcomes": all_outcomes,
        }
        with open(AGGREGATE_FILE, "w") as f:
            json.dump(aggregate, f, indent=2, default=str)
        
        # Write human-readable summary
        _write_summary(all_stats, all_outcomes)
        
        print(f"\n  Progress saved. {shard+1}/{TOTAL_SHARDS} shards complete.")
        sys.stdout.flush()
    
    total_elapsed = time.time() - total_start
    print(f"\n{'='*60}")
    print(f"SWEEP COMPLETE in {total_elapsed/3600:.1f}h")
    print(f"See: {SUMMARY_FILE}")
    print(f"{'='*60}")

def _write_summary(all_stats, all_outcomes):
    """Write a human-readable summary file."""
    total_caught = sum(s.get("caught", 0) for s in all_stats)
    total_missed = sum(s.get("missed", 0) for s in all_stats)
    total_unviable = sum(s.get("unviable", 0) for s in all_stats)
    total_timeout = sum(s.get("timeout", 0) for s in all_stats)
    total_tested = total_caught + total_missed + total_unviable + total_timeout
    kill_rate = (total_caught / (total_caught + total_missed) * 100) if (total_caught + total_missed) > 0 else 0
    
    # Aggregate file-level stats
    all_miss = Counter()
    all_caught_f = Counter()
    for s in all_stats:
        for k, v in s.get("miss_by_file", {}).items():
            all_miss[k] += v
        for k, v in s.get("caught_by_file", {}).items():
            all_caught_f[k] += v
    
    with open(SUMMARY_FILE, "w") as f:
        f.write(f"Terrain Mutation Testing Sweep - {datetime.now().strftime('%Y-%m-%d %H:%M')}\n")
        f.write(f"{'='*60}\n")
        f.write(f"Shards completed: {len(all_stats)}/{TOTAL_SHARDS}\n")
        f.write(f"Total tested: {total_tested}\n")
        f.write(f"Caught: {total_caught}\n")
        f.write(f"Missed: {total_missed}\n")
        f.write(f"Unviable: {total_unviable}\n")
        f.write(f"Timeout: {total_timeout}\n")
        f.write(f"Raw Kill Rate: {kill_rate:.1f}%\n\n")
        
        f.write(f"Per-shard results:\n")
        f.write(f"{'Shard':>6} {'Caught':>8} {'Missed':>8} {'Unviable':>8} {'Time(m)':>8}\n")
        for s in all_stats:
            f.write(f"  {s['shard']:>4} {s.get('caught',0):>8} {s.get('missed',0):>8} {s.get('unviable',0):>8} {s.get('elapsed_s',0)/60:>8.1f}\n")
        
        f.write(f"\nMisses by file:\n")
        for k, v in all_miss.most_common():
            short = k.split("/")[-1]
            f.write(f"  {short}: {v}\n")
        
        f.write(f"\nCaught by file:\n")
        for k, v in all_caught_f.most_common():
            short = k.split("/")[-1]
            f.write(f"  {short}: {v}\n")

if __name__ == "__main__":
    main()
