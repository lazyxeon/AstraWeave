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
# One node process reads the downloaded bytes once. It hashes those bytes,
# gunzips that buffer, classifies every header, and writes the tree from
# that buffer. A size or checksum field that is not plain octal is refused.
# A non-zero name prefix is refused. Links and devices are refused.
# After extraction, an lstat walk rejects a symlink or any entry that is not
# a regular file or a directory. Cleanup does not chmod a symbolic link. The
# archive must contain exactly one top-level entry, a real directory named
# for this release asset. The installed name is sccache.exe on win32 and
# sccache otherwise. That entry must be one regular file with a link count
# of 1. The script copies that file alone and checks the tool cache holds
# only that file. The installed bytes are compared to the named member of
# the already-verified archive.
#
# Workflow steps start this script with /usr/bin/env -i and an absolute bash
# --noprofile --norc. Darwin uses /bin/bash. Every other OS uses
# /usr/bin/bash. The command passes HOME, GITHUB_OUTPUT, RUNNER_TOOL_CACHE,
# and RUNNER_OS. It does not pass PATH. The job environment and the step
# shell are trusted until /usr/bin/env starts. curl, mktemp, rm, chmod,
# mkdir, sleep, and cp are absolute paths for Darwin, Linux, or
# Windows Git Bash. cygpath is an absolute path and runs only when
# RUNNER_OS is Windows. A missing tool fails closed. Nothing is resolved
# from PATH. node is an absolute path. echo and printf are shell builtins.
# curl's first argument is -q, so it does not read a curlrc.
# --release-base is for the policy test harness. Workflows call this
# script with no arguments.
# SCCACHE_READER_CHECK, when set, runs the archive reader on that path and
# exits with the reader's status. The fetch command does not pass it.
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
  if [ -n "$dest" ] && [ -n "${RM_BIN:-}" ]; then
    "$RM_BIN" -rf "$dest" "${dest}.complete" || true
  fi
  write_output "available=false" || true
  exit 0
}

# A closed pipe delivers SIGPIPE. Remove a partial install and exit 141
# before a marker or available=true can count as success.
on_sigpipe() {
  if [ -n "$dest" ] && [ -n "${RM_BIN:-}" ]; then
    "$RM_BIN" -rf "$dest" "${dest}.complete" || true
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

NODE_BIN=""
for _aw_node in "/opt/homebrew/bin/node" "/usr/local/bin/node" "/usr/bin/node" "/c/Program Files/nodejs/node.exe"; do
  if [ -x "$_aw_node" ]; then
    NODE_BIN="$_aw_node"
    break
  fi
done
if [ -z "$NODE_BIN" ]; then
  fail_closed "node is required to match the action tool-cache arch"
fi
if [ -z "${RUNNER_TOOL_CACHE:-}" ]; then
  fail_closed "RUNNER_TOOL_CACHE is unset"
fi

node_arch="$("$NODE_BIN" -p 'process.arch')" || fail_closed "could not read the runner arch"
node_platform="$("$NODE_BIN" -p 'process.platform')" || fail_closed "could not read the runner platform"

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

# First executable in the argument list. Arguments are absolute paths.
# A relative path is skipped. PATH is not searched. A miss returns 1.
pick_tool() {
  local _aw_pick
  for _aw_pick in "$@"; do
    case "$_aw_pick" in
      /*) ;;
      *) continue ;;
    esac
    if [ -x "$_aw_pick" ]; then
      printf '%s\n' "$_aw_pick"
      return 0
    fi
  done
  return 1
}

# Darwin uses /bin for rm, chmod, mkdir, sleep, and cp, and /usr/bin for
# curl and mktemp. Linux uses /usr/bin, then /bin. Windows Git Bash
# uses /usr/bin with and without .exe. curl may live under /mingw64/bin.
# cygpath runs only when RUNNER_OS is Windows, by an absolute path.
case "$node_platform" in
  darwin)
    CURL_BIN="$(pick_tool /usr/bin/curl)" || fail_closed "curl is missing"
    MKTEMP_BIN="$(pick_tool /usr/bin/mktemp)" || fail_closed "mktemp is missing"
    RM_BIN="$(pick_tool /bin/rm /usr/bin/rm)" || fail_closed "rm is missing"
    CHMOD_BIN="$(pick_tool /bin/chmod /usr/bin/chmod)" || fail_closed "chmod is missing"
    MKDIR_BIN="$(pick_tool /bin/mkdir /usr/bin/mkdir)" || fail_closed "mkdir is missing"
    SLEEP_BIN="$(pick_tool /bin/sleep /usr/bin/sleep)" || fail_closed "sleep is missing"
    CP_BIN="$(pick_tool /bin/cp /usr/bin/cp)" || fail_closed "cp is missing"
    ;;
  win32)
    CURL_BIN="$(pick_tool /mingw64/bin/curl.exe /usr/bin/curl.exe /usr/bin/curl)" || fail_closed "curl is missing"
    MKTEMP_BIN="$(pick_tool /usr/bin/mktemp /usr/bin/mktemp.exe)" || fail_closed "mktemp is missing"
    RM_BIN="$(pick_tool /usr/bin/rm /usr/bin/rm.exe)" || fail_closed "rm is missing"
    CHMOD_BIN="$(pick_tool /usr/bin/chmod /usr/bin/chmod.exe)" || fail_closed "chmod is missing"
    MKDIR_BIN="$(pick_tool /usr/bin/mkdir /usr/bin/mkdir.exe)" || fail_closed "mkdir is missing"
    SLEEP_BIN="$(pick_tool /usr/bin/sleep /usr/bin/sleep.exe)" || fail_closed "sleep is missing"
    CP_BIN="$(pick_tool /usr/bin/cp /usr/bin/cp.exe)" || fail_closed "cp is missing"
    ;;
  linux)
    CURL_BIN="$(pick_tool /usr/bin/curl)" || fail_closed "curl is missing"
    MKTEMP_BIN="$(pick_tool /usr/bin/mktemp)" || fail_closed "mktemp is missing"
    RM_BIN="$(pick_tool /usr/bin/rm /bin/rm)" || fail_closed "rm is missing"
    CHMOD_BIN="$(pick_tool /usr/bin/chmod /bin/chmod)" || fail_closed "chmod is missing"
    MKDIR_BIN="$(pick_tool /usr/bin/mkdir /bin/mkdir)" || fail_closed "mkdir is missing"
    SLEEP_BIN="$(pick_tool /usr/bin/sleep /bin/sleep)" || fail_closed "sleep is missing"
    CP_BIN="$(pick_tool /usr/bin/cp /bin/cp)" || fail_closed "cp is missing"
    ;;
  *)
    fail_closed "unsupported platform ${node_platform}"
    ;;
esac

tool_cache="$RUNNER_TOOL_CACHE"
if [ "${RUNNER_OS:-}" = "Windows" ]; then
  CYGPATH_BIN="$(pick_tool /usr/bin/cygpath.exe /usr/bin/cygpath)" || fail_closed "cygpath is missing"
  tool_cache="$("$CYGPATH_BIN" -u "$RUNNER_TOOL_CACHE")" || fail_closed "could not convert the tool cache path"
fi

sha256_file() {
  "$NODE_BIN" -e 'const fs=require("fs"),crypto=require("crypto");const h=crypto.createHash("sha256");h.update(fs.readFileSync(process.argv[1]));process.stdout.write(h.digest("hex"));' "$1"
}

# One regular file. lstat does not follow a link, and a link count above 1
# is another name for the same inode.
plain_binary() {
  "$NODE_BIN" -e 'const fs=require("fs"); let st; try { st=fs.lstatSync(process.argv[1]); } catch (e) { process.exit(1); } if (!st.isFile() || st.nlink !== 1) process.exit(1);' "$1"
}

# lstat does not follow a link. A symlink to a directory is not a directory.
plain_directory() {
  "$NODE_BIN" -e 'const fs=require("fs"); let st; try { st=fs.lstatSync(process.argv[1]); } catch (e) { process.exit(1); } if (!st.isDirectory()) process.exit(1);' "$1"
}

# Reject a symlink or any other non-regular entry anywhere under the root.
# A failed directory read or a failed entry lstat rejects the tree.
plain_tree() {
  "$NODE_BIN" -e 'const fs=require("fs"),path=require("path"); const root=process.argv[1]; function walk(dir){ let names; try { names=fs.readdirSync(dir); } catch (e) { process.exit(1); } for (const name of names) { const p=path.join(dir, name); let st; try { st=fs.lstatSync(p); } catch (e) { process.exit(1); } if (st.isDirectory()) { walk(p); continue; } if (!st.isFile()) process.exit(1); } } walk(root);' "$1"
}

# Deny by default. Type flags: NUL and "0" are regular files, "5" is a
# directory. A name that ends in "/" is directory-like for NUL and "0" too.
# Directory-like headers must have size zero; the following block is the
# next header. A header this reader will not classify is refused. Anything
# else, including links and devices, is refused. Exit 1 is a refused member.
# Exit 2 is an archive this parser cannot read. The hash mode prints the
# sha256 of one named member from these bytes.
read_archive() {
  "$NODE_BIN" -e 'const fs=require("fs"),zlib=require("zlib"),crypto=require("crypto"); const archive=process.argv[1]; const mode=process.argv[2]; const want=process.argv[3]||""; const destDir=process.argv[4]||""; let raw; try { raw=fs.readFileSync(archive); } catch (e) { process.exit(2); } const fileHash=crypto.createHash("sha256").update(raw).digest("hex"); if (mode==="stage") process.stdout.write(fileHash+"\n"); let buf; try { buf=zlib.gunzipSync(raw); } catch (e) { process.exit(2); } const staged=[]; let memberHash=""; function plainOctal(header, start, length) { const bytes=header.subarray(start, start+length); let i=0; while (i<bytes.length && bytes[i]===32) i++; if (i>=bytes.length) return 0; if (bytes[i]===0) { for (let j=i; j<bytes.length; j++) if (bytes[j]!==0) return null; return 0; } let value=0; let saw=false; for (; i<bytes.length; i++) { const c=bytes[i]; if (c===0 || c===32) { for (let j=i; j<bytes.length; j++) if (bytes[j]!==0 && bytes[j]!==32) return null; return saw?value:0; } if (c<48 || c>55) return null; saw=true; value=value*8+(c-48); if (!Number.isSafeInteger(value)) return null; } return saw?value:0; } function nameEndsWithSlash(header) { let end=0; while (end<100 && header[end]!==0) end++; return end>0 && header[end-1]===47; } function nameLength(header) { let end=0; while (end<100 && header[end]!==0) end++; return end; } function isUstar(header) { return header[257]===117 && header[258]===115 && header[259]===116 && header[260]===97 && header[261]===114; } function prefixLength(header) { let end=0; while (end<155 && header[345+end]!==0) end++; return end; } function memberName(header) { const n=nameLength(header); let name=""; for (let i=0;i<n;i++) name+=String.fromCharCode(header[i]); return name; } let off=0; while (off+512<=buf.length) { const h=buf.subarray(off, off+512); let zero=true; for (let i=0;i<512;i++) if (h[i]!==0) { zero=false; break; } if (zero) { if (mode==="hash") process.exit(2); if (mode==="stage") { const path=require("path"); for (const item of staged) { if (item.dir) continue; const out=path.join(destDir, item.name); fs.mkdirSync(path.dirname(out), {recursive:true}); fs.writeFileSync(out, item.bytes, {mode: item.mode & 511}); } for (const item of staged) { if (!item.dir) continue; const out=path.join(destDir, item.name.replace(/\/+$/, "")); fs.mkdirSync(out, {recursive:true}); fs.chmodSync(out, item.mode & 511); } if (memberHash) process.stdout.write(memberHash+"\n"); process.exit(0); } process.exit(0); } let sum=0; for (let i=0;i<512;i++) sum+=(i>=148 && i<156)?32:h[i]; const recorded=plainOctal(h, 148, 8); if (recorded===null || recorded!==sum) process.exit(2); if (nameLength(h)===0) process.exit(1); if (prefixLength(h)!==0) process.exit(1); const typ=h[156]; const size=plainOctal(h, 124, 12); if (size===null || size<0) process.exit(2); const directoryLike=typ===53 || ((typ===0 || typ===48) && nameEndsWithSlash(h)); if (directoryLike) { if (size!==0) process.exit(1); if (mode==="stage") { const perm=plainOctal(h, 100, 8); if (perm===null) process.exit(2); staged.push({name:memberName(h), dir:true, mode:perm}); } off+=512; continue; } if (!(typ===0 || typ===48)) process.exit(1); const blocks=Math.ceil(size/512); const dataOff=off+512; if (mode==="stage") { if (dataOff+size>buf.length) process.exit(2); const perm=plainOctal(h, 100, 8); if (perm===null) process.exit(2); const bytes=Buffer.from(buf.subarray(dataOff, dataOff+size)); const name=memberName(h); staged.push({name:name, bytes:bytes, mode:perm}); if (name===want) memberHash=crypto.createHash("sha256").update(bytes).digest("hex"); } if (mode==="hash" && memberName(h)===want) { if (dataOff+size>buf.length) process.exit(2); const hsh=crypto.createHash("sha256"); hsh.update(buf.subarray(dataOff, dataOff+size)); process.stdout.write(hsh.digest("hex")); process.exit(0); } off+=512*(1+blocks); if (off>buf.length) process.exit(2); } process.exit(2);' "$@"
}

archive_plain_members() {
  read_archive "$1" members
}

archive_member_sha256() {
  read_archive "$1" hash "$2"
}

# An extracted member can be mode 000. chmod the directory before descending
# so the temp tree can be removed. A symbolic link is skipped before any
# other test, and is never chmodded: chmod would follow it. A failure here
# cannot change the exit.
restore_tree() {
  if [ -L "$1" ]; then
    return 0
  fi
  "$CHMOD_BIN" u+rwx "$1" 2>/dev/null || true
  local child
  shopt -s nullglob dotglob
  for child in "$1"/*; do
    if [ -L "$child" ]; then
      continue
    fi
    if [ -d "$child" ]; then
      restore_tree "$child"
    else
      "$CHMOD_BIN" u+rwx "$child" 2>/dev/null || true
    fi
  done
  shopt -u nullglob dotglob
}

cleanup_tmp() {
  if [ -z "${tmp:-}" ] || [ ! -d "$tmp" ]; then
    return 0
  fi
  restore_tree "$tmp" || true
  "$RM_BIN" -rf "$tmp" || true
}

if [ -n "${SCCACHE_READER_CHECK:-}" ]; then
  reader_status=0
  archive_plain_members "$SCCACHE_READER_CHECK" || reader_status=$?
  exit "$reader_status"
fi

tmp="$("$MKTEMP_BIN" -d)" || fail_closed "could not create a temp directory"
trap cleanup_tmp EXIT

download_archive() {
  local attempt=1 delay="$DELAY"
  while [ "$attempt" -le "$ATTEMPTS" ]; do
    # -q is first so curl does not read a curlrc from HOME or CURL_HOME.
    if "$CURL_BIN" -q -fsSL --retry 0 --connect-timeout 20 --max-time 180 -o "$tmp/pkg.tar.gz" "$url"; then
      return 0
    fi
    "$RM_BIN" -f "$tmp/pkg.tar.gz"
    if [ "$attempt" -eq "$ATTEMPTS" ]; then
      return 1
    fi
    echo "sccache prefetch attempt ${attempt}/${ATTEMPTS} failed; retrying in ${delay}s" >&2
    "$SLEEP_BIN" "$delay"
    delay=$((delay * 2))
    attempt=$((attempt + 1))
  done
  return 1
}

if ! download_archive; then
  fail_closed "release download failed after ${ATTEMPTS} attempts (${url})"
fi

release_root="${filename%.tar.gz}"
"$RM_BIN" -rf "$tmp/extract"
"$MKDIR_BIN" -p "$tmp/extract" || fail_closed "could not create the extract directory"
stage_status=0
stage_out="$(read_archive "$tmp/pkg.tar.gz" stage "${release_root}/${binary_name}" "$tmp/extract")" || stage_status=$?
got="${stage_out%%$'\n'*}"
member_sum=""
if [ "$stage_out" != "$got" ]; then
  member_sum="${stage_out#*$'\n'}"
  member_sum="${member_sum%%$'\n'*}"
fi
want="$(pinned_sha256 "$filename")"
if [ "${#want}" -ne 64 ] || { [ "${#got}" -eq 64 ] && [ "$got" != "$want" ]; }; then
  trap - ERR
  builtin printf 'sccache checksum mismatch for %s\n' "$filename" >&2
  exit 1
fi
if [ "$stage_status" -eq 1 ]; then
  fail_closed "archive has a symlink or non-regular entry"
fi
if [ "$stage_status" -ne 0 ] || [ "${#got}" -ne 64 ]; then
  fail_closed "could not extract the archive"
fi

if ! plain_tree "$tmp/extract"; then
  fail_closed "archive has a symlink or non-regular entry"
fi
top_name="$("$NODE_BIN" -e 'const fs=require("fs"); const names=fs.readdirSync(process.argv[1]); if (names.length!==1) process.exit(1); process.stdout.write(names[0]);' "$tmp/extract")" || fail_closed "archive does not contain exactly one release root"
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
"$RM_BIN" -rf "$dest" "${dest}.complete"
if ! {
  "$MKDIR_BIN" -p "$dest" \
    && "$CP_BIN" "${inner}/${binary_name}" "${dest}/${binary_name}"
}; then
  fail_closed "could not install sccache into the tool cache"
fi

if ! plain_binary "$dest/$binary_name"; then
  fail_closed "tool cache is missing the sccache binary"
fi
if ! "$NODE_BIN" -e 'const fs=require("fs"); const names=fs.readdirSync(process.argv[1]); if (names.length!==1 || names[0]!==process.argv[2]) process.exit(1);' "$dest" "$binary_name"; then
  fail_closed "tool cache is missing the sccache binary"
fi
dst_sum="$(sha256_file "${dest}/${binary_name}")"
if [ -z "$member_sum" ] || [ "$member_sum" != "$dst_sum" ]; then
  fail_closed "tool cache is missing the sccache binary"
fi

echo "sccache ${VERSION} cached at ${dest}" || true
write_output "available=true"
if ! printf '' > "${dest}.complete"; then
  "$RM_BIN" -rf "$dest" "${dest}.complete" || true
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    printf 'available=false\n' > "$GITHUB_OUTPUT" || true
  fi
  exit 0
fi
