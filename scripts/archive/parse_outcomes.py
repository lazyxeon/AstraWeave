#!/usr/bin/env python3
"""Parse cargo-mutants outcomes.json and write summary to a text file."""
import json
import sys
from collections import Counter

def parse_outcomes(json_path, output_path):
    with open(json_path) as f:
        data = json.load(f)

    items = data["outcomes"]
    total = data["total_mutants"]
    caught = data["caught"]
    missed = data["missed"]
    unviable = data["unviable"]
    timeout = data["timeout"]

    with open(output_path, "w") as out:
        out.write(f"total={total} caught={caught} missed={missed} unviable={unviable} timeout={timeout}\n")

        miss_files = Counter(
            x["scenario"]["Mutant"]["file"]
            for x in items if x["summary"] == "MissedMutant"
        )
        out.write("MISSES:\n")
        for k, v in miss_files.most_common():
            out.write(f"  {k}: {v}\n")

        caught_files = Counter(
            x["scenario"]["Mutant"]["file"]
            for x in items if x["summary"] == "CaughtMutant"
        )
        out.write("CAUGHT:\n")
        for k, v in caught_files.most_common():
            out.write(f"  {k}: {v}\n")

        unv_files = Counter(
            x["scenario"]["Mutant"]["file"]
            for x in items if x["summary"] == "Unviable"
        )
        out.write("UNVIABLE:\n")
        for k, v in unv_files.most_common():
            out.write(f"  {k}: {v}\n")

    print(f"DONE: {output_path}")

if __name__ == "__main__":
    json_path = sys.argv[1] if len(sys.argv) > 1 else r"C:\temp\mutants.out\outcomes.json"
    output_path = sys.argv[2] if len(sys.argv) > 2 else "shard_results.txt"
    parse_outcomes(json_path, output_path)
