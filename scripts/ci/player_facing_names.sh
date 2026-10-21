#!/usr/bin/env bash
# Fail when player-facing asset text says "Veilweaver".
# Scanned: dialogue, quests, echoes, voices, npc lines, and cinematic
# cards (including the title-card notes in the .ron files).
# Cell tags and editor metadata under assets/cells are not player text.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

mapfile -t files < <(
  find \
    assets/dialogue_intro.toml \
    assets/quests_main.toml \
    assets/echoes.toml \
    assets/voices.toml \
    assets/npc \
    assets/cinematics \
    -type f \( -name '*.toml' -o -name '*.ron' -o -name '*.txt' \) | sort
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
