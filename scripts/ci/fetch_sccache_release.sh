#!/usr/bin/env bash
# Prefetch the pinned sccache release into the Actions tool cache.
#
# mozilla-actions/sccache-action v0.0.11 (commit fc920bf0ec8de6ee65d409111f7ec508035751ba)
# calls @actions/tool-cache find('sccache', version) before downloadTool.
# find() hits when $RUNNER_TOOL_CACHE/sccache/<semver.clean(version)>/<process.arch>
# exists beside an empty "<arch>.complete" marker. semver.clean("v0.19.0") is
# "0.19.0". Seeding that directory skips the action's single GitHub Releases
# fetch, which returns HTTP 504 often enough to fail the job (#721).
#
# Exhausted retries exit 0 and set available=false. The workflow fallback step
# clears the compiler wrapper. This script does not export compiler settings.
set -euo pipefail

VERSION=v0.19.0
ATTEMPTS="${SCCACHE_FETCH_ATTEMPTS:-5}"
DELAY="${SCCACHE_FETCH_DELAY_SECONDS:-2}"
RELEASE_BASE="${SCCACHE_RELEASE_BASE:-https://github.com/mozilla/sccache/releases/download/${VERSION}}"

write_output() {
  if [ -z "${GITHUB_OUTPUT:-}" ]; then
    echo "GITHUB_OUTPUT is unset; not recording $1" >&2
    return 0
  fi
  printf '%s\n' "$1" >> "$GITHUB_OUTPUT"
}

fail_closed() {
  echo "sccache prefetch failed: $*" >&2
  write_output "available=false"
  exit 0
}

case "$ATTEMPTS" in
  ''|*[!0-9]*) fail_closed "SCCACHE_FETCH_ATTEMPTS must be a positive integer" ;;
esac
case "$DELAY" in
  ''|*[!0-9]*) fail_closed "SCCACHE_FETCH_DELAY_SECONDS must be a non-negative integer" ;;
esac
if [ "$ATTEMPTS" -lt 1 ]; then
  fail_closed "SCCACHE_FETCH_ATTEMPTS must be at least 1"
fi

if ! command -v node >/dev/null 2>&1; then
  fail_closed "node is required to match the action tool-cache arch"
fi
if [ -z "${RUNNER_TOOL_CACHE:-}" ]; then
  fail_closed "RUNNER_TOOL_CACHE is unset"
fi

node_arch="$(node -p 'process.arch')"
node_platform="$(node -p 'process.platform')"

case "$node_arch" in
  x64) asset_arch=x86_64 ;;
  arm64) asset_arch=aarch64 ;;
  arm) asset_arch=armv7 ;;
  *) fail_closed "unsupported arch ${node_arch}" ;;
esac

case "$node_platform" in
  darwin) asset_os=apple-darwin ;;
  win32) asset_os=pc-windows-msvc ;;
  linux)
    if [ "$node_arch" = "arm" ]; then
      asset_os=unknown-linux-musleabi
    else
      asset_os=unknown-linux-musl
    fi
    ;;
  *) fail_closed "unsupported platform ${node_platform}" ;;
esac

case "$VERSION" in
  v[0-9]*.[0-9]*.[0-9]*) clean_version="${VERSION#v}" ;;
  *) fail_closed "refusing version ${VERSION}" ;;
esac
case "$clean_version" in
  *[!0-9.]*) fail_closed "refusing version ${VERSION}" ;;
esac

filename="sccache-${VERSION}-${asset_arch}-${asset_os}.tar.gz"
url="${RELEASE_BASE}/${filename}"
sum_url="${url}.sha256"

tool_cache="$RUNNER_TOOL_CACHE"
if command -v cygpath >/dev/null 2>&1; then
  tool_cache="$(cygpath -u "$RUNNER_TOOL_CACHE")"
fi

sha256_file() {
  node -e 'const fs=require("fs"),crypto=require("crypto");const h=crypto.createHash("sha256");h.update(fs.readFileSync(process.argv[1]));process.stdout.write(h.digest("hex"));' "$1"
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

prefetch_once() {
  rm -f "$tmp/pkg.tar.gz" "$tmp/pkg.sha256"
  curl -fsSL --retry 0 --connect-timeout 20 --max-time 180 -o "$tmp/pkg.tar.gz" "$url" || return 1
  curl -fsSL --retry 0 --connect-timeout 20 --max-time 180 -o "$tmp/pkg.sha256" "$sum_url" || return 1
  local got want
  got="$(sha256_file "$tmp/pkg.tar.gz")"
  want="$(tr -d '[:space:]' < "$tmp/pkg.sha256")"
  if [ "${#want}" -ne 64 ] || [ "$got" != "$want" ]; then
    echo "sccache checksum mismatch for ${filename}" >&2
    return 1
  fi
}

attempt=1
delay="$DELAY"
while [ "$attempt" -le "$ATTEMPTS" ]; do
  if prefetch_once; then
    break
  fi
  if [ "$attempt" -eq "$ATTEMPTS" ]; then
    fail_closed "release download failed after ${ATTEMPTS} attempts (${url})"
  fi
  echo "sccache prefetch attempt ${attempt}/${ATTEMPTS} failed; retrying in ${delay}s" >&2
  sleep "$delay"
  delay=$((delay * 2))
  attempt=$((attempt + 1))
done

rm -rf "$tmp/extract"
mkdir -p "$tmp/extract"
tar -xzf "$tmp/pkg.tar.gz" -C "$tmp/extract"

inner=""
for candidate in "$tmp/extract"/*; do
  if [ -d "$candidate" ]; then
    inner="$candidate"
    break
  fi
done
if [ -z "$inner" ]; then
  fail_closed "archive has no top-level directory"
fi
if [ ! -f "$inner/sccache" ] && [ ! -f "$inner/sccache.exe" ]; then
  fail_closed "archive has no sccache binary"
fi

dest="${tool_cache}/sccache/${clean_version}/${node_arch}"
rm -rf "$dest" "${dest}.complete"
mkdir -p "$dest"
cp -R "${inner}/." "${dest}/"
printf '' > "${dest}.complete"

if [ ! -f "$dest/sccache" ] && [ ! -f "$dest/sccache.exe" ]; then
  fail_closed "tool cache is missing the sccache binary"
fi
if [ ! -f "${dest}.complete" ]; then
  fail_closed "tool cache is missing the .complete marker"
fi

write_output "available=true"
echo "sccache ${VERSION} cached at ${dest}"
