#!/usr/bin/env bash
# Fetch enough history for `git merge-base origin/$base_ref HEAD` without a full clone.
set -euo pipefail

base_ref="${1:?base ref name required (e.g. main)}"
remote_base="refs/remotes/origin/${base_ref}"
fetch_base="+refs/heads/${base_ref}:${remote_base}"

git fetch --no-tags --filter=blob:none origin "${fetch_base}"

attempt=0
max_attempts=20
while ! git merge-base "${remote_base}" HEAD >/dev/null 2>&1; do
  attempt=$((attempt + 1))
  if [ "${attempt}" -gt "${max_attempts}" ]; then
    break
  fi
  git fetch --deepen=50 --no-tags --filter=blob:none origin HEAD "${fetch_base}"
done

if ! git merge-base "${remote_base}" HEAD >/dev/null 2>&1; then
  echo "deepen cap hit; unshallowing" >&2
  git fetch --unshallow --no-tags --filter=blob:none origin HEAD "${fetch_base}" || true
fi

git merge-base "${remote_base}" HEAD || {
  echo "merge-base between origin/${base_ref} and HEAD unresolved after full fetch" >&2
  exit 1
}
