#!/usr/bin/env python3
"""
Analyze completed terrain sweep results and generate a classification report.
Reads the aggregate.json produced by scripts/run_terrain_sweep.py and generates
a detailed markdown report.
"""
import json
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path

WORKSPACE = r"C:\Users\pv2br\AstraWeave-AI-Native-Gaming-Engine"
RESULTS_DIR = os.path.join(WORKSPACE, "terrain_sweep_results")
AGGREGATE_FILE = os.path.join(RESULTS_DIR, "aggregate.json")

# Files known to be low-observability lookup tables
LOOKUP_TABLE_FILES = {
    "astraweave-terrain/src/marching_cubes_tables.rs",
}

# Files that are binary entrypoints (no lib test coverage)
BINARY_FILES = set()  # Terrain has none

# Files behind non-default feature gates
FEATURE_GATED_FILES = set()  # Terrain has no feature-gated files

def load_aggregate():
    with open(AGGREGATE_FILE) as f:
        return json.load(f)

def analyze(data):
    outcomes = data.get("all_outcomes", [])
    shard_stats = data.get("shard_stats", [])
    
    # Aggregate by file and summary
    file_results = defaultdict(lambda: Counter())
    file_functions = defaultdict(lambda: defaultdict(list))
    
    for o in outcomes:
        summary = o["summary"]
        mutant = o["scenario"]["Mutant"]
        src_file = mutant["file"]
        func = mutant.get("function", {}).get("function_name", "unknown")
        name = mutant.get("name", "unknown")
        
        file_results[src_file][summary] += 1
        if summary == "MissedMutant":
            file_functions[src_file][func].append(name)
    
    return file_results, file_functions, shard_stats

def classify_file(src_file):
    """Classify a file as lookup-table, binary, feature-gated, or testable."""
    if src_file in LOOKUP_TABLE_FILES:
        return "lookup-table"
    if src_file in BINARY_FILES:
        return "binary"
    if src_file in FEATURE_GATED_FILES:
        return "feature-gated"
    return "testable"

def generate_report(data, file_results, file_functions, shard_stats):
    lines = []
    
    # Header
    completed = data.get("completed_shards", 0)
    total_shards = data.get("total_shards", 22)
    
    total_caught = sum(s.get("caught", 0) for s in shard_stats)
    total_missed = sum(s.get("missed", 0) for s in shard_stats)
    total_unviable = sum(s.get("unviable", 0) for s in shard_stats)
    total_timeout = sum(s.get("timeout", 0) for s in shard_stats)
    total_tested = total_caught + total_missed + total_unviable + total_timeout
    
    raw_rate = (total_caught / (total_caught + total_missed) * 100) if (total_caught + total_missed) > 0 else 0
    
    lines.append("# Terrain Mutation Testing — Sweep Analysis Report\n")
    lines.append(f"**Shards**: {completed}/{total_shards}")
    lines.append(f"**Total Tested**: {total_tested}")
    lines.append(f"**Caught**: {total_caught}")
    lines.append(f"**Missed**: {total_missed}")
    lines.append(f"**Unviable**: {total_unviable}")
    lines.append(f"**Timeout**: {total_timeout}")
    lines.append(f"**Raw Kill Rate**: {raw_rate:.1f}%\n")
    
    # Classification summary
    lookup_missed = 0
    testable_missed = 0
    testable_caught = 0
    lookup_caught = 0
    
    for src_file, counts in file_results.items():
        cat = classify_file(src_file)
        if cat == "lookup-table":
            lookup_missed += counts.get("MissedMutant", 0)
            lookup_caught += counts.get("CaughtMutant", 0)
        else:
            testable_missed += counts.get("MissedMutant", 0)
            testable_caught += counts.get("CaughtMutant", 0)
    
    adj_rate = (testable_caught / (testable_caught + testable_missed) * 100) if (testable_caught + testable_missed) > 0 else 0
    
    lines.append("## Classification Summary\n")
    lines.append(f"| Category | Caught | Missed | Kill Rate |")
    lines.append(f"|----------|:------:|:------:|:---------:|")
    lines.append(f"| Testable code | {testable_caught} | {testable_missed} | {adj_rate:.1f}% |")
    lines.append(f"| Lookup tables | {lookup_caught} | {lookup_missed} | (excluded) |")
    lines.append(f"| **Adjusted Rate** | | | **{adj_rate:.1f}%** |\n")
    
    # Per-file breakdown
    lines.append("## Per-File Results\n")
    lines.append("| File | Caught | Missed | Unviable | Kill Rate | Category |")
    lines.append("|------|:------:|:------:|:--------:|:---------:|----------|")
    
    for src_file in sorted(file_results.keys(), key=lambda f: file_results[f].get("MissedMutant", 0), reverse=True):
        counts = file_results[src_file]
        caught = counts.get("CaughtMutant", 0)
        missed = counts.get("MissedMutant", 0)
        unviable = counts.get("Unviable", 0)
        cat = classify_file(src_file)
        rate = (caught / (caught + missed) * 100) if (caught + missed) > 0 else 0
        short = src_file.split("/")[-1]
        lines.append(f"| {short} | {caught} | {missed} | {unviable} | {rate:.0f}% | {cat} |")
    
    # Top missed functions
    lines.append("\n## Top Missed Functions (Remediation Targets)\n")
    lines.append("| File | Function | Missed | Example Mutations |")
    lines.append("|------|----------|:------:|-------------------|")
    
    all_funcs = []
    for src_file, funcs in file_functions.items():
        cat = classify_file(src_file)
        if cat != "testable":
            continue
        for func_name, mutations in funcs.items():
            all_funcs.append((src_file, func_name, len(mutations), mutations[:3]))
    
    all_funcs.sort(key=lambda x: x[2], reverse=True)
    
    for src_file, func_name, count, examples in all_funcs[:30]:
        short = src_file.split("/")[-1]
        ex_strs = "; ".join(e[:60] for e in examples)
        lines.append(f"| {short} | `{func_name}` | {count} | {ex_strs} |")
    
    # Shard timing summary
    lines.append("\n## Shard Timing\n")
    lines.append("| Shard | Caught | Missed | Unviable | Time (min) |")
    lines.append("|:-----:|:------:|:------:|:--------:|:----------:|")
    
    for s in shard_stats:
        shard = s.get("shard", "?")
        caught = s.get("caught", 0)
        missed = s.get("missed", 0)
        unviable = s.get("unviable", 0)
        elapsed = s.get("elapsed_s", 0) / 60
        lines.append(f"| {shard} | {caught} | {missed} | {unviable} | {elapsed:.1f} |")
    
    return "\n".join(lines)

def main():
    if not os.path.exists(AGGREGATE_FILE):
        print(f"ERROR: {AGGREGATE_FILE} not found. Run sweep first.")
        sys.exit(1)
    
    data = load_aggregate()
    file_results, file_functions, shard_stats = analyze(data)
    report = generate_report(data, file_results, file_functions, shard_stats)
    
    output_path = os.path.join(RESULTS_DIR, "analysis_report.md")
    with open(output_path, "w") as f:
        f.write(report)
    
    print(f"Report written to: {output_path}")
    print(report[:500])  # Preview

if __name__ == "__main__":
    main()
