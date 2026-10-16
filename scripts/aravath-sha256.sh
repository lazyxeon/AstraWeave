#!/usr/bin/env bash
# Write SHA256SUMS.txt for every regular file in a staged Aravath folder.
# The checksum file itself is not included. Uses sha256sum when it exists,
# otherwise Python, so the Windows Git Bash job and Linux both work.
set -euo pipefail

dir=${1:-dist/aravath_greybox}
cd "$dir"

hash_one() {
  local file=$1
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file"
  else
    python -c 'import hashlib, pathlib, sys; p = pathlib.Path(sys.argv[1]); print(hashlib.sha256(p.read_bytes()).hexdigest() + "  " + p.as_posix())' "$file"
  fi
}

tmp=$(mktemp)
find . -type f ! -name SHA256SUMS.txt | sort | while read -r file; do
  hash_one "${file#./}"
done >"$tmp"
mv "$tmp" SHA256SUMS.txt
echo "wrote $(pwd)/SHA256SUMS.txt ($(wc -l < SHA256SUMS.txt) files)"
