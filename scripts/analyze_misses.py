import json
import sys

with open(r'C:\temp\mutants.out\outcomes.json') as f:
    data = json.load(f)

misses = [o for o in data['outcomes'] if o['summary'] == 'MissedMutant']
print(f"Total misses: {len(misses)}")
print("=" * 80)

# Group by file and function
from collections import defaultdict
groups = defaultdict(list)
for m in misses:
    sc = m['scenario']['Mutant']
    fn_name = sc['function'].get('function_name', 'unknown')
    key = f"{sc['file'].split('/')[-1]}::{fn_name}"
    groups[key].append(sc['replacement'])

for key in sorted(groups.keys()):
    reps = groups[key]
    print(f"\n{key} ({len(reps)} misses):")
    for r in reps[:10]:
        print(f"  -> {r}")
    if len(reps) > 10:
        print(f"  ... and {len(reps)-10} more")
