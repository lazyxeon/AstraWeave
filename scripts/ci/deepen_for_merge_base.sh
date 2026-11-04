#!/usr/bin/env bash
# Fetch enough history for `git merge-base origin/$base_ref HEAD` without a full clone.
set -euo pipefail

base_ref="${1:?base ref name required (e.g. main)}"

git fetch --no-tags --filter=blob:none origin "${base_ref}"

attempt=0
max_attempts=200
while ! git merge-base "origin/${base_ref}" HEAD >/dev/null 2>&1; do
  attempt=$((attempt + 1))
  if [ "${attempt}" -gt "${max_attempts}" ]; then
    echo "merge-base between origin/${base_ref} and HEAD still unresolved after ${max_attempts} deepen steps" >&2
    exit 1
  fi
  git fetch --deepen=50 --no-tags --filter=blob:none origin HEAD "${base_ref}"
done

git merge-base "origin/${base_ref}" HEAD
