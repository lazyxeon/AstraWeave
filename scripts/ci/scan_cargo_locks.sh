#!/usr/bin/env bash
# Scan every tracked Cargo.lock with cargo-audit or cargo-deny.
#
# `git ls-files '**/Cargo.lock'` matches nested locks only. The pathspec
# requires a slash, so the root Cargo.lock is a separate spec. New locks
# are picked up by one of those two specs; the policy test fails if a
# tracked Cargo.lock is left out.
#
# Usage: scan_cargo_locks.sh audit|deny [extra args passed to the tool...]
set -euo pipefail

mode="${1:?usage: scan_cargo_locks.sh audit|deny [extra args...]}"
shift

cd "$(dirname "$0")/../.."

case "$mode" in
  audit|deny) ;;
  *)
    echo "unknown mode: ${mode} (expected audit or deny)" >&2
    exit 1
    ;;
esac

mapfile -t locks < <(git ls-files -- '**/Cargo.lock' 'Cargo.lock' | sort -u)
if [ "${#locks[@]}" -eq 0 ]; then
  echo "git ls-files returned no Cargo.lock" >&2
  exit 1
fi

for lock in "${locks[@]}"; do
  case "$mode" in
    audit)
      echo "::group::cargo audit ${lock}"
      cargo audit --file "$lock" "$@"
      echo "::endgroup::"
      ;;
    deny)
      manifest="$(dirname -- "$lock")/Cargo.toml"
      if [ ! -f "$manifest" ]; then
        echo "missing ${manifest} for ${lock}" >&2
        exit 1
      fi
      echo "::group::cargo deny ${lock}"
      # Extra args are check options (for example --hide-inclusion-graph).
      cargo deny --manifest-path "$manifest" check "$@" bans sources advisories licenses
      echo "::endgroup::"
      ;;
  esac
done
