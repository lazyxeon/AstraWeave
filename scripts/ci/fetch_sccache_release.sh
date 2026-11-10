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
# The archive sha256 values are pinned below. A mismatch exits 1 immediately:
# no retry and no fallback, including when removing the temp directory fails.
# Any other failure exits 0 and records available=false when the step output
# can be written. A failure to write that output still exits 0 and does not
# record available=true. A failed download retries with backoff. The workflow
# fallback step clears the compiler wrapper. This script does not export
# compiler settings.
#
# --release-base is for the policy test harness. Workflows call this script
# with no arguments.
# -E runs the ERR trap inside functions and command substitutions.
set -eEuo pipefail

VERSION=v0.19.0
ATTEMPTS="${SCCACHE_FETCH_ATTEMPTS:-5}"
DELAY="${SCCACHE_FETCH_DELAY_SECONDS:-2}"
release_base="https://github.com/mozilla/sccache/releases/download/${VERSION}"
if [ "${1:-}" = "--release-base" ]; then
  release_base="${2:?sccache prefetch: --release-base needs a URL}"
fi

write_output() {
  if [ -z "${GITHUB_OUTPUT:-}" ]; then
    echo "GITHUB_OUTPUT is unset; not recording $1" >&2
    return 0
  fi
  printf '%s\n' "$1" >> "$GITHUB_OUTPUT"
}

fail_closed() {
  echo "sccache prefetch failed: $*" >&2
  write_output "available=false" || true
  exit 0
}

# A checksum mismatch is the only path that exits non-zero. It disarms this
# trap before exit 1, so a cleanup failure cannot fall back. A failure in a
# command substitution exits 1 so the parent shell, not the subshell, runs
# the fallback. Writing the step output and removing the temp directory
# cannot change a fallback exit.
on_unexpected_error() {
  if [ "${BASH_SUBSHELL:-0}" -ne 0 ]; then
    exit 1
  fi
  trap - ERR
  fail_closed "unexpected error"
}
trap on_unexpected_error ERR

# sha256 of the v0.19.0 release archives this script can install.
pinned_sha256() {
  case "$1" in
    sccache-v0.19.0-x86_64-unknown-linux-musl.tar.gz)
      printf '%s\n' 'aa73c7270c997894038c0d8f0d297865f57569e1687883322bc431713d4bb9dd'
      ;;
    sccache-v0.19.0-aarch64-unknown-linux-musl.tar.gz)
      printf '%s\n' '3e47fc4a71b1ddd1b4294a0de9d34498922fdf41e3cd0bda5df3954e3d6deb9d'
      ;;
    sccache-v0.19.0-armv7-unknown-linux-musleabi.tar.gz)
      printf '%s\n' '2a478c2e5006af97b7cfe08b3b4cb02a3770b06aade91f8c03fc99b05edba041'
      ;;
    sccache-v0.19.0-x86_64-apple-darwin.tar.gz)
      printf '%s\n' '2db7344bb96de350df79cd9f0124dd4e9baa47b3de2fac1e63e1346b66b20684'
      ;;
    sccache-v0.19.0-aarch64-apple-darwin.tar.gz)
      printf '%s\n' 'a35a0b7d622618774bc1bb8102d72562dfb21e619f88b153b370d7f89bc286af'
      ;;
    sccache-v0.19.0-x86_64-pc-windows-msvc.tar.gz)
      printf '%s\n' '130e4f8eb672e76590e9993c2ab890c4763b88238577bb11d8062158c7698353'
      ;;
    sccache-v0.19.0-aarch64-pc-windows-msvc.tar.gz)
      printf '%s\n' 'a680e8e6058b033cd6ff56d56b2219ca13f1c1a45f50fb3b04f4ed80851f5c2b'
      ;;
    *)
      return 1
      ;;
  esac
}

case "$ATTEMPTS" in
  ''|*[!0-9]*) fail_closed "SCCACHE_FETCH_ATTEMPTS must be a positive integer" ;;
esac
case "$DELAY" in
  ''|*[!0-9]*) fail_closed "SCCACHE_FETCH_DELAY_SECONDS must be a non-negative integer" ;;
esac
# Base 10. A leading zero would otherwise abort $((delay * 2)).
ATTEMPTS=$((10#$ATTEMPTS))
DELAY=$((10#$DELAY))
if [ "$ATTEMPTS" -lt 1 ]; then
  fail_closed "SCCACHE_FETCH_ATTEMPTS must be at least 1"
fi

if ! command -v node >/dev/null 2>&1; then
  fail_closed "node is required to match the action tool-cache arch"
fi
if [ -z "${RUNNER_TOOL_CACHE:-}" ]; then
  fail_closed "RUNNER_TOOL_CACHE is unset"
fi

node_arch="$(node -p 'process.arch')" || fail_closed "could not read the runner arch"
node_platform="$(node -p 'process.platform')" || fail_closed "could not read the runner platform"

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
url="${release_base}/${filename}"
if ! pinned_sha256 "$filename" >/dev/null; then
  fail_closed "no pinned checksum for ${filename}"
fi

tool_cache="$RUNNER_TOOL_CACHE"
if command -v cygpath >/dev/null 2>&1; then
  tool_cache="$(cygpath -u "$RUNNER_TOOL_CACHE")"
fi

sha256_file() {
  node -e 'const fs=require("fs"),crypto=require("crypto");const h=crypto.createHash("sha256");h.update(fs.readFileSync(process.argv[1]));process.stdout.write(h.digest("hex"));' "$1"
}

# One regular file. lstat does not follow a link, and a link count above 1
# is another name for the same inode.
plain_binary() {
  node -e 'const fs=require("fs"); let st; try { st=fs.lstatSync(process.argv[1]); } catch (e) { process.exit(1); } if (!st.isFile() || st.nlink !== 1) process.exit(1);' "$1"
}

tmp="$(mktemp -d)" || fail_closed "could not create a temp directory"
trap 'rm -rf "$tmp" || true' EXIT

download_archive() {
  local attempt=1 delay="$DELAY"
  while [ "$attempt" -le "$ATTEMPTS" ]; do
    if curl -fsSL --retry 0 --connect-timeout 20 --max-time 180 -o "$tmp/pkg.tar.gz" "$url"; then
      return 0
    fi
    rm -f "$tmp/pkg.tar.gz"
    if [ "$attempt" -eq "$ATTEMPTS" ]; then
      return 1
    fi
    echo "sccache prefetch attempt ${attempt}/${ATTEMPTS} failed; retrying in ${delay}s" >&2
    sleep "$delay"
    delay=$((delay * 2))
    attempt=$((attempt + 1))
  done
  return 1
}

if ! download_archive; then
  fail_closed "release download failed after ${ATTEMPTS} attempts (${url})"
fi

got="$(sha256_file "$tmp/pkg.tar.gz")"
want="$(pinned_sha256 "$filename")"
if [ "${#want}" -ne 64 ] || [ "$got" != "$want" ]; then
  echo "sccache checksum mismatch for ${filename}" >&2
  trap - ERR
  exit 1
fi

rm -rf "$tmp/extract"
mkdir -p "$tmp/extract" || fail_closed "could not create the extract directory"
tar -xzf "$tmp/pkg.tar.gz" -C "$tmp/extract" || fail_closed "could not extract the archive"

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
if ! plain_binary "$inner/sccache" && ! plain_binary "$inner/sccache.exe"; then
  fail_closed "archive has no sccache binary"
fi

dest="${tool_cache}/sccache/${clean_version}/${node_arch}"
rm -rf "$dest" "${dest}.complete"
if ! {
  mkdir -p "$dest" \
    && cp -R "${inner}/." "${dest}/"
}; then
  rm -rf "$dest" "${dest}.complete"
  fail_closed "could not install sccache into the tool cache"
fi

if ! plain_binary "$dest/sccache" && ! plain_binary "$dest/sccache.exe"; then
  rm -rf "$dest" "${dest}.complete"
  fail_closed "tool cache is missing the sccache binary"
fi
if ! printf '' > "${dest}.complete"; then
  rm -rf "$dest" "${dest}.complete"
  fail_closed "tool cache is missing the .complete marker"
fi
if [ ! -f "${dest}.complete" ]; then
  rm -rf "$dest" "${dest}.complete"
  fail_closed "tool cache is missing the .complete marker"
fi

write_output "available=true"
echo "sccache ${VERSION} cached at ${dest}"
