#!/usr/bin/env bash
# Fail when player-facing asset text says "Veilweaver".
# Roots are walked recursively: dialogue, quests, echoes, voices, recipes,
# zones, npc lines, cinematic cards, and cell copy. Tooling manifests and
# binary packs are not scanned.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

mapfile -t files < <(
  find \
    assets/dialogue_intro.toml \
    assets/dialogue_audio_map.toml \
    assets/quests_main.toml \
    assets/echoes.toml \
    assets/voices.toml \
    assets/recipes.toml \
    assets/zones.json \
    assets/npc \
    assets/cinematics \
    assets/cells \
    -type f \( -name '*.toml' -o -name '*.ron' -o -name '*.txt' -o -name '*.json' \) \
    | sort
)

fail=0
for file in "${files[@]}"; do
  if grep -In -E -i 'veilweaver' "$file"; then
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "Player-facing assets must not say Veilweaver. Use in-world wording (the Loom, threads, Aravath)." >&2
  exit 1
fi

echo "Player-facing assets do not say Veilweaver (${#files[@]} files)."
