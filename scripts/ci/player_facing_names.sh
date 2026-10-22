#!/usr/bin/env bash
# Fail when player-facing text says "Veilweaver".
# Asset roots are walked recursively. A window-title scan reads quoted
# strings in the visual demo. Find and grep errors fail the scan. Fewer
# than MIN_PLAYER_FACING_FILES matches fails the scan.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

MIN_PLAYER_FACING_FILES=20
TITLE_FILE=examples/veilweaver_demo/src/visual_renderer.rs

asset_paths=(
  assets/dialogue_intro.toml
  assets/dialogue_audio_map.toml
  assets/quests_main.toml
  assets/echoes.toml
  assets/voices.toml
  assets/recipes.toml
  assets/zones.json
  assets/npc
  assets/cinematics
  assets/cells
)

# Scan $1 for the retired name.
# 0 clean, 1 hit, 2 grep could not read the file.
check_text() {
  local file=$1
  local status=0
  grep -In -E -i 'veilweaver' "$file" || status=$?
  if [ "$status" -eq 2 ]; then
    echo "player-facing scan: grep failed on $file" >&2
    return 2
  fi
  if [ "$status" -eq 0 ]; then
    return 1
  fi
  return 0
}

# Quoted strings only, so type names such as VeilweaverApp are not hits.
# 0 clean, 1 hit, 2 grep could not read the file.
check_quotes() {
  local file=$1
  local status=0
  grep -nE '"[^"]*[Vv]eilweaver[^"]*"' "$file" || status=$?
  if [ "$status" -eq 2 ]; then
    echo "player-facing scan: quoted-title grep failed on $file" >&2
    return 2
  fi
  if [ "$status" -eq 0 ]; then
    return 1
  fi
  return 0
}

# Print matches under the asset roots of $1.
# 0 clean, 1 content or too few files, 2 find or grep failed.
scan_assets() {
  local base=$1
  local minimum=$2
  local list status count fail file
  list=$(mktemp)
  status=0
  find "${asset_paths[@]/#/$base/}" \
    -type f \( -name '*.toml' -o -name '*.ron' -o -name '*.txt' -o -name '*.json' \) \
    -print0 >"$list" || status=$?
  if [ "$status" -ne 0 ]; then
    rm -f "$list"
    echo "player-facing scan: find failed ($status) under $base" >&2
    return 2
  fi
  count=0
  fail=0
  while IFS= read -r -d '' file; do
    count=$((count + 1))
    status=0
    check_text "$file" || status=$?
    if [ "$status" -eq 2 ]; then
      rm -f "$list"
      return 2
    fi
    if [ "$status" -eq 1 ]; then
      fail=1
    fi
  done <"$list"
  rm -f "$list"
  if [ "$count" -lt "$minimum" ]; then
    echo "player-facing scan found $count files under $base, need at least $minimum" >&2
    return 1
  fi
  if [ "$fail" -ne 0 ]; then
    echo "Player-facing assets must not say Veilweaver. Use in-world wording (the Loom, threads, Aravath)." >&2
    return 1
  fi
  echo "Player-facing assets do not say Veilweaver ($count files)."
  return 0
}

self_test() {
  local tmp status
  tmp=$(mktemp -d)
  mkdir -p "$tmp/missing"

  status=0
  scan_assets "$tmp/missing" 1 || status=$?
  if [ "$status" -ne 2 ]; then
    echo "self-test: a missing asset tree should be a scan error, got $status" >&2
    rm -rf "$tmp"
    return 1
  fi

  mkdir -p "$tmp/empty/assets/npc" "$tmp/empty/assets/cinematics" "$tmp/empty/assets/cells"
  : >"$tmp/empty/assets/dialogue_intro.toml"
  : >"$tmp/empty/assets/dialogue_audio_map.toml"
  : >"$tmp/empty/assets/quests_main.toml"
  : >"$tmp/empty/assets/echoes.toml"
  : >"$tmp/empty/assets/voices.toml"
  : >"$tmp/empty/assets/recipes.toml"
  : >"$tmp/empty/assets/zones.json"
  status=0
  scan_assets "$tmp/empty" 20 || status=$?
  if [ "$status" -eq 0 ]; then
    echo "self-test: seven files should be too few" >&2
    rm -rf "$tmp"
    return 1
  fi

  mkdir -p "$tmp/dironly"
  status=0
  check_text "$tmp/dironly" || status=$?
  if [ "$status" -ne 2 ]; then
    echo "self-test: grep on a directory should be a scan error, got $status" >&2
    rm -rf "$tmp"
    return 1
  fi

  printf '%s\n' '        .with_title("Veilweaver — AI-Native Game Engine")' >"$tmp/title.rs"
  status=0
  check_quotes "$tmp/title.rs" || status=$?
  if [ "$status" -ne 1 ]; then
    echo "self-test: the window title should be a hit, got $status" >&2
    rm -rf "$tmp"
    return 1
  fi

  printf '%s\n' '        .with_title("AstraWeave")' >"$tmp/clean.rs"
  status=0
  check_quotes "$tmp/clean.rs" || status=$?
  if [ "$status" -ne 0 ]; then
    echo "self-test: AstraWeave title should be clean, got $status" >&2
    rm -rf "$tmp"
    return 1
  fi

  rm -rf "$tmp"
}

self_test

scan_assets "$root" "$MIN_PLAYER_FACING_FILES"

status=0
check_quotes "$TITLE_FILE" || status=$?
if [ "$status" -eq 2 ]; then
  exit 2
fi
if [ "$status" -eq 1 ]; then
  echo "Player-facing window text must not say Veilweaver ($TITLE_FILE)." >&2
  exit 1
fi
