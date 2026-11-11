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
# no retry, no fallback, no available= line, and no tool cache. The mismatch
# line is builtin printf after the ERR trap is disarmed, so a write error on
# that line still exits non-zero. Removing the temp directory cannot change
# that exit.
#
# Every other handled failure exits 0, removes the tool-cache directory and
# its .complete marker when that directory was created, and records
# available=false when the step output can be written. A failure to write
# that output still exits 0 and does not record available=true. A write error
# on stdout or stderr cannot change the exit. A closed pipe is SIGPIPE
# (exit 141): the tool cache is removed and the .complete marker is not left
# behind. available=true is written only after the install checks. The marker
# is written only after that. If that write fails, the tool cache is
# removed and the step output is rewritten to available=false. A partial
# or failed run leaves no marker.
# A failure to remove the temp directory cannot change the exit or the
# recorded output. A failed download retries with backoff.
#
# Before extraction, archive headers must be regular files or directories.
# A directory-like header must have size zero. A size or checksum field that
# is not plain octal is refused, so the reader and the extractor agree on
# the next header. Links and devices are refused, including when tar
# materializes a link as a copy. After extraction, an lstat walk rejects a
# symlink or any entry that is not a regular file or a directory. Cleanup
# does not chmod a symbolic link. The archive must contain exactly one
# top-level entry, a real directory named for this release asset. The
# installed name is sccache.exe on win32 and sccache otherwise. That entry
# must be one regular file with a link count of 1. The script copies that
# file alone, checks the tool cache holds only that file, and checks its
# bytes match the extracted file.
#
# Workflow steps start this script with /usr/bin/env -i and bash
# --noprofile --norc, passing PATH, HOME, GITHUB_OUTPUT, and
# RUNNER_TOOL_CACHE. The job environment is trusted until /usr/bin/env
# starts. curl's first argument is -q, so it does not read a curlrc.
# --release-base is for the policy test harness. Workflows call this script
# with no arguments.
# -E runs the ERR trap inside functions and command substitutions.
set -eEuo pipefail

VERSION=v0.19.0
ATTEMPTS="${SCCACHE_FETCH_ATTEMPTS:-5}"
DELAY="${SCCACHE_FETCH_DELAY_SECONDS:-2}"
release_base="https://github.com/mozilla/sccache/releases/download/${VERSION}"
dest=""
binary_name=""

write_output() {
  if [ -z "${GITHUB_OUTPUT:-}" ]; then
    echo "GITHUB_OUTPUT is unset; not recording $1" >&2 || true
    return 0
  fi
  printf '%s\n' "$1" >> "$GITHUB_OUTPUT"
}

fail_closed() {
  echo "sccache prefetch failed: $*" >&2 || true
  if [ -n "$dest" ]; then
    rm -rf "$dest" "${dest}.complete" || true
  fi
  write_output "available=false" || true
  exit 0
}

# A closed pipe delivers SIGPIPE. Remove a partial install and exit 141
# before a marker or available=true can count as success.
on_sigpipe() {
  if [ -n "$dest" ]; then
    rm -rf "$dest" "${dest}.complete" || true
  fi
  exit 141
}
trap on_sigpipe PIPE

# A checksum mismatch is the only path that exits non-zero on purpose. It
# disarms this trap before builtin printf, so a write error on that message
# still exits non-zero and cannot fall back. The EXIT cleanup cannot fail
# (`|| true`). A failure in a command substitution exits 1 so the parent
# shell, not the subshell, runs the fallback. Writing the step output, the
# diagnostic lines, and removing the temp directory or the tool cache cannot
# change a fallback exit.
on_unexpected_error() {
  if [ "${BASH_SUBSHELL:-0}" -ne 0 ]; then
    exit 1
  fi
  trap - ERR
  fail_closed "unexpected error"
}
trap on_unexpected_error ERR

if [ "${1:-}" = "--release-base" ]; then
  if [ -z "${2:-}" ]; then
    fail_closed "--release-base needs a URL"
  fi
  release_base="$2"
fi

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
  darwin)
    asset_os=apple-darwin
    binary_name=sccache
    ;;
  win32)
    asset_os=pc-windows-msvc
    binary_name=sccache.exe
    ;;
  linux)
    binary_name=sccache
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

# lstat does not follow a link. A symlink to a directory is not a directory.
plain_directory() {
  node -e 'const fs=require("fs"); let st; try { st=fs.lstatSync(process.argv[1]); } catch (e) { process.exit(1); } if (!st.isDirectory()) process.exit(1);' "$1"
}

# Reject a symlink or any other non-regular entry anywhere under the root.
# A failed directory read or a failed entry lstat rejects the tree.
plain_tree() {
  node -e 'const fs=require("fs"),path=require("path"); const root=process.argv[1]; function walk(dir){ let names; try { names=fs.readdirSync(dir); } catch (e) { process.exit(1); } for (const name of names) { const p=path.join(dir, name); let st; try { st=fs.lstatSync(p); } catch (e) { process.exit(1); } if (st.isDirectory()) { walk(p); continue; } if (!st.isFile()) process.exit(1); } } walk(root);' "$1"
}

# Deny by default. Type flags: NUL and "0" are regular files, "5" is a
# directory. A name that ends in "/" is directory-like for NUL and "0" too.
# Directory-like headers must have size zero; the following block is the
# next header. Anything else, including links and devices, is refused.
# Exit 1 is a refused member. Exit 2 is an archive this parser cannot read.
archive_plain_members() {
  node -e 'const fs=require("fs"),zlib=require("zlib"); let buf; try { buf=zlib.gunzipSync(fs.readFileSync(process.argv[1])); } catch (e) { process.exit(2); } function plainOctal(header, start, length) { const bytes=header.subarray(start, start+length); let i=0; while (i<bytes.length && bytes[i]===32) i++; if (i>=bytes.length) return 0; if (bytes[i]===0) { for (let j=i; j<bytes.length; j++) if (bytes[j]!==0) return null; return 0; } let value=0; let saw=false; for (; i<bytes.length; i++) { const c=bytes[i]; if (c===0 || c===32) { for (let j=i; j<bytes.length; j++) if (bytes[j]!==0 && bytes[j]!==32) return null; return saw?value:0; } if (c<48 || c>55) return null; saw=true; value=value*8+(c-48); if (!Number.isSafeInteger(value)) return null; } return saw?value:0; } function nameEndsWithSlash(header) { let end=0; while (end<100 && header[end]!==0) end++; return end>0 && header[end-1]===47; } let off=0; while (off+512<=buf.length) { const h=buf.subarray(off, off+512); let zero=true; for (let i=0;i<512;i++) if (h[i]!==0) { zero=false; break; } if (zero) process.exit(0); let sum=0; for (let i=0;i<512;i++) sum+=(i>=148 && i<156)?32:h[i]; const recorded=plainOctal(h, 148, 8); if (recorded===null || recorded!==sum) process.exit(2); const typ=h[156]; const size=plainOctal(h, 124, 12); if (size===null || size<0) process.exit(2); const directoryLike=typ===53 || ((typ===0 || typ===48) && nameEndsWithSlash(h)); if (directoryLike) { if (size!==0) process.exit(1); off+=512; continue; } if (!(typ===0 || typ===48)) process.exit(1); const blocks=Math.ceil(size/512); off+=512*(1+blocks); if (off>buf.length) process.exit(2); } process.exit(2);' "$1"
}

# An extracted member can be mode 000. chmod the directory before descending
# so the temp tree can be removed. A symbolic link is skipped before any
# other test, and is never chmodded: chmod would follow it. A failure here
# cannot change the exit.
restore_tree() {
  if [ -L "$1" ]; then
    return 0
  fi
  chmod u+rwx "$1" 2>/dev/null || true
  local child
  shopt -s nullglob dotglob
  for child in "$1"/*; do
    if [ -L "$child" ]; then
      continue
    fi
    if [ -d "$child" ]; then
      restore_tree "$child"
    else
      chmod u+rwx "$child" 2>/dev/null || true
    fi
  done
  shopt -u nullglob dotglob
}

cleanup_tmp() {
  if [ -z "${tmp:-}" ] || [ ! -d "$tmp" ]; then
    return 0
  fi
  restore_tree "$tmp" || true
  rm -rf "$tmp" || true
}

tmp="$(mktemp -d)" || fail_closed "could not create a temp directory"
trap cleanup_tmp EXIT

download_archive() {
  local attempt=1 delay="$DELAY"
  while [ "$attempt" -le "$ATTEMPTS" ]; do
    # -q is first so curl does not read a curlrc from HOME or CURL_HOME.
    if curl -q -fsSL --retry 0 --connect-timeout 20 --max-time 180 -o "$tmp/pkg.tar.gz" "$url"; then
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
  trap - ERR
  builtin printf 'sccache checksum mismatch for %s\n' "$filename" >&2
  exit 1
fi

member_status=0
archive_plain_members "$tmp/pkg.tar.gz" || member_status=$?
if [ "$member_status" -eq 1 ]; then
  fail_closed "archive has a symlink or non-regular entry"
fi
if [ "$member_status" -ne 0 ]; then
  fail_closed "could not extract the archive"
fi

rm -rf "$tmp/extract"
mkdir -p "$tmp/extract" || fail_closed "could not create the extract directory"
tar -xzf "$tmp/pkg.tar.gz" -C "$tmp/extract" || fail_closed "could not extract the archive"

if ! plain_tree "$tmp/extract"; then
  fail_closed "archive has a symlink or non-regular entry"
fi

release_root="${filename%.tar.gz}"
top_name="$(node -e 'const fs=require("fs"); const names=fs.readdirSync(process.argv[1]); if (names.length!==1) process.exit(1); process.stdout.write(names[0]);' "$tmp/extract")" || fail_closed "archive does not contain exactly one release root"
if [ "$top_name" != "$release_root" ]; then
  fail_closed "archive release root does not match the asset"
fi
inner="$tmp/extract/$release_root"
if ! plain_directory "$inner"; then
  fail_closed "archive release root is not a directory"
fi
if ! plain_binary "$inner/$binary_name"; then
  fail_closed "archive has no sccache binary"
fi

dest="${tool_cache}/sccache/${clean_version}/${node_arch}"
rm -rf "$dest" "${dest}.complete"
if ! {
  mkdir -p "$dest" \
    && cp "${inner}/${binary_name}" "${dest}/${binary_name}"
}; then
  fail_closed "could not install sccache into the tool cache"
fi

if ! plain_binary "$dest/$binary_name"; then
  fail_closed "tool cache is missing the sccache binary"
fi
if ! node -e 'const fs=require("fs"); const names=fs.readdirSync(process.argv[1]); if (names.length!==1 || names[0]!==process.argv[2]) process.exit(1);' "$dest" "$binary_name"; then
  fail_closed "tool cache is missing the sccache binary"
fi
src_sum="$(sha256_file "${inner}/${binary_name}")"
dst_sum="$(sha256_file "${dest}/${binary_name}")"
if [ "$src_sum" != "$dst_sum" ]; then
  fail_closed "tool cache is missing the sccache binary"
fi

echo "sccache ${VERSION} cached at ${dest}" || true
write_output "available=true"
if ! printf '' > "${dest}.complete"; then
  rm -rf "$dest" "${dest}.complete" || true
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    printf 'available=false\n' > "$GITHUB_OUTPUT" || true
  fi
  exit 0
fi
