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
# The shell reads the download once, hashes that stream with /usr/bin/sha256sum
# or /usr/bin/shasum, and keeps the same bytes. A mismatch exits 1. Node is
# given that buffer on stdin. It hashes the buffer and parses that same
# buffer before any mkdir, chmod, or write. A mismatch exits before a member
# is written. When the hash matches, the process accepts the release layout
# and writes only the one binary at the expected name. LICENSE and README.md
# may be present and are not written. A size or mode field is refused unless every byte is an octal
# digit or a trailing NUL. A space-padded size is refused. A checksum field
# is the same, plus the ustar trailer of a NUL followed by spaces. A
# high-bit size is refused. A non-zero name prefix is refused. Links, devices, extra names, a leading
# slash, and ".." are refused, and nothing is written. After that write, an
# lstat walk rejects a symlink or any entry that is not a regular file or a
# directory. Cleanup does not chmod a symbolic link, including when the temp
# directory itself is a symbolic link. The installed name is sccache.exe on
# win32 and sccache otherwise. That file must be one regular file with a
# link count of 1. The script copies that file alone and checks the tool
# cache holds only that file. The shell hashes the installed file with
# /usr/bin/sha256sum or /usr/bin/shasum and compares it to the member digest
# pinned below.
#
# Workflow steps start this script with /usr/bin/env -i and an absolute bash
# --noprofile --norc. Darwin uses /bin/bash. Every other OS uses
# /usr/bin/bash. The command passes HOME, GITHUB_OUTPUT, RUNNER_TOOL_CACHE,
# and RUNNER_OS. It does not pass PATH. The job environment and the step
# shell are trusted until /usr/bin/env starts. curl, mktemp, rm, chmod,
# mkdir, sleep, cp, and tee are absolute paths for Darwin, Linux, or
# Windows Git Bash. Windows curl is /c/Windows/System32/curl.exe.
# cygpath is an absolute path and runs only when RUNNER_OS is Windows.
# A missing tool fails closed. Nothing is resolved from PATH. node is one
# absolute path per OS: /usr/local/bin/node on Linux, /opt/homebrew/bin/node
# on Darwin, and /c/Program Files/nodejs/node.exe on Windows Git Bash.
# echo and printf are shell builtins. curl's first argument is -q, so it
# does not read a curlrc. --release-base is for the policy test harness.
# Workflows call this script with no arguments.
# SCCACHE_READER_CHECK, when set, runs the archive reader on that path and
# exits with the reader's status. SCCACHE_STAGE_CHECK, when set, runs the
# stage reader on that file's bytes and exits with its status. The fetch
# command passes neither.
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

# sha256 of the sccache binary inside each pinned archive.
pinned_member_sha256() {
  case "$1" in
    sccache-v0.19.0-x86_64-unknown-linux-musl.tar.gz)
      printf '%s\n' '0b44834be857e21b026f7db49a7762023fedb5eb5ed60725a66f698d8d79a34b'
      ;;
    sccache-v0.19.0-aarch64-unknown-linux-musl.tar.gz)
      printf '%s\n' 'c192919f21426298532538710488c6c2f0784516ee8b34bb971fb003963526b4'
      ;;
    sccache-v0.19.0-armv7-unknown-linux-musleabi.tar.gz)
      printf '%s\n' 'a7d867dda28d27b5e87d042e708236d7eb4c288949d4737e3f79aea3d37e53d3'
      ;;
    sccache-v0.19.0-x86_64-apple-darwin.tar.gz)
      printf '%s\n' '3cfabdf4d282e8c83cf98ff935cf368a73efab42c70c3efeaa14cfe5083a310e'
      ;;
    sccache-v0.19.0-aarch64-apple-darwin.tar.gz)
      printf '%s\n' 'd81ba04c41e6d772f7ecc83fe8f26f2f8b25f2cf3a730049c926c7592c819290'
      ;;
    sccache-v0.19.0-x86_64-pc-windows-msvc.tar.gz)
      printf '%s\n' 'ba5542107b245431e37011110c1929d635e3a8e52cc98d35c4b01d0aa328b5a5'
      ;;
    sccache-v0.19.0-aarch64-pc-windows-msvc.tar.gz)
      printf '%s\n' 'b31f677701b0a1ddc1790c4b5705d2ad7f6f9ba5f80a559c369b61172358dbf4'
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

host_kernel="$("/usr/bin/uname" -s)" || fail_closed "node is required to match the action tool-cache arch"
case "$host_kernel" in
  Linux)
    NODE_BIN="/usr/local/bin/node"
    ;;
  Darwin)
    NODE_BIN="/opt/homebrew/bin/node"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    NODE_BIN="/c/Program Files/nodejs/node.exe"
    ;;
  *)
    fail_closed "node is required to match the action tool-cache arch"
    ;;
esac
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
if ! pinned_member_sha256 "$filename" >/dev/null; then
  fail_closed "no pinned checksum for ${filename}"
fi

# First executable in the argument list. Callers pass absolute paths.
# PATH is not searched. A miss returns 1.
pick_tool() {
  local _aw_pick
  for _aw_pick in "$@"; do
    if [ -x "$_aw_pick" ]; then
      printf '%s\n' "$_aw_pick"
      return 0
    fi
  done
  return 1
}

# Darwin uses /bin for rm, chmod, mkdir, sleep, and cp, and /usr/bin for
# curl and mktemp. Linux uses /usr/bin, then /bin. Windows Git Bash uses
# /usr/bin with and without .exe. Windows curl is only
# /c/Windows/System32/curl.exe. cygpath runs only when RUNNER_OS is
# Windows, by an absolute path.
case "$node_platform" in
  darwin)
    CURL_BIN="$(pick_tool /usr/bin/curl)" || fail_closed "curl is missing"
    MKTEMP_BIN="$(pick_tool /usr/bin/mktemp)" || fail_closed "mktemp is missing"
    RM_BIN="$(pick_tool /bin/rm /usr/bin/rm)" || fail_closed "rm is missing"
    CHMOD_BIN="$(pick_tool /bin/chmod /usr/bin/chmod)" || fail_closed "chmod is missing"
    MKDIR_BIN="$(pick_tool /bin/mkdir /usr/bin/mkdir)" || fail_closed "mkdir is missing"
    SLEEP_BIN="$(pick_tool /bin/sleep /usr/bin/sleep)" || fail_closed "sleep is missing"
    CP_BIN="$(pick_tool /bin/cp /usr/bin/cp)" || fail_closed "cp is missing"
    TEE_BIN="$(pick_tool /usr/bin/tee /bin/tee)" || fail_closed "tee is missing"
    ;;
  win32)
    CURL_BIN="/c/Windows/System32/curl.exe"
    if [ ! -x "$CURL_BIN" ]; then
      fail_closed "curl is missing"
    fi
    MKTEMP_BIN="$(pick_tool /usr/bin/mktemp /usr/bin/mktemp.exe)" || fail_closed "mktemp is missing"
    RM_BIN="$(pick_tool /usr/bin/rm /usr/bin/rm.exe)" || fail_closed "rm is missing"
    CHMOD_BIN="$(pick_tool /usr/bin/chmod /usr/bin/chmod.exe)" || fail_closed "chmod is missing"
    MKDIR_BIN="$(pick_tool /usr/bin/mkdir /usr/bin/mkdir.exe)" || fail_closed "mkdir is missing"
    SLEEP_BIN="$(pick_tool /usr/bin/sleep /usr/bin/sleep.exe)" || fail_closed "sleep is missing"
    CP_BIN="$(pick_tool /usr/bin/cp /usr/bin/cp.exe)" || fail_closed "cp is missing"
    TEE_BIN="$(pick_tool /usr/bin/tee /usr/bin/tee.exe /bin/tee)" || fail_closed "tee is missing"
    ;;
  linux)
    CURL_BIN="$(pick_tool /usr/bin/curl)" || fail_closed "curl is missing"
    MKTEMP_BIN="$(pick_tool /usr/bin/mktemp)" || fail_closed "mktemp is missing"
    RM_BIN="$(pick_tool /usr/bin/rm /bin/rm)" || fail_closed "rm is missing"
    CHMOD_BIN="$(pick_tool /usr/bin/chmod /bin/chmod)" || fail_closed "chmod is missing"
    MKDIR_BIN="$(pick_tool /usr/bin/mkdir /bin/mkdir)" || fail_closed "mkdir is missing"
    SLEEP_BIN="$(pick_tool /usr/bin/sleep /bin/sleep)" || fail_closed "sleep is missing"
    CP_BIN="$(pick_tool /usr/bin/cp /bin/cp)" || fail_closed "cp is missing"
    TEE_BIN="$(pick_tool /usr/bin/tee /bin/tee)" || fail_closed "tee is missing"
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

# Hash with /usr/bin/sha256sum or /usr/bin/shasum -a 256. A missing flag on
# shasum is not this hash. Absolute paths only.
_shell_sha256_parse() {
  local out="$1"
  printf '%s\n' "${out%%[[:space:]]*}"
}

shell_sha256() {
  local out
  if [ -x /usr/bin/sha256sum ]; then
    out="$(/usr/bin/sha256sum -- "$1")" || return 1
  elif [ -x /usr/bin/shasum ]; then
    out="$(/usr/bin/shasum -a 256 -- "$1")" || return 1
  else
    return 1
  fi
  _shell_sha256_parse "$out"
}

shell_sha256_stdin() {
  local out
  if [ -x /usr/bin/sha256sum ]; then
    out="$(/usr/bin/sha256sum)" || return 1
  elif [ -x /usr/bin/shasum ]; then
    out="$(/usr/bin/shasum -a 256)" || return 1
  else
    return 1
  fi
  _shell_sha256_parse "$out"
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
# Directory-like headers must have size zero. A header this reader will not
# classify is refused. The stage mode compares the file hash to the pin
# before it creates a directory or writes a byte. It writes only the binary
# named by the caller, into a fresh directory from mktemp.
read_archive() {
  "$NODE_BIN" -e 'const fs = require("fs");
const zlib = require("zlib");
const crypto = require("crypto");
const path = require("path");
const { execFileSync } = require("child_process");
const archive = process.argv[1];
const mode = process.argv[2];
const wantName = process.argv[3];
const pin = process.argv[4];
const mktempBin = process.argv[5];
const parentDir = process.argv[6];
let raw;
try { raw = archive === "-" ? fs.readFileSync(0) : fs.readFileSync(archive); } catch (e) { process.exit(2); }
const fileHash = crypto.createHash("sha256").update(raw).digest("hex");
function finish(code, reason) {
  if (mode === "stage" && fileHash !== pin) {
    process.stdout.write(fileHash + "\n");
    process.exit(3);
  }
  if (code !== 0) {
    if (mode === "stage") {
      process.stdout.write(fileHash + "\n");
      if (reason) process.stdout.write(reason + "\n");
    }
    process.exit(code);
  }
}
let buf;
try { buf = zlib.gunzipSync(raw); } catch (e) { finish(2, ""); }
function plainOctal(header, start, length, posixTrailer) {
  const bytes = header.subarray(start, start + length);
  let value = 0;
  let saw = false;
  for (let i = 0; i < bytes.length; i++) {
    const c = bytes[i];
    if (c === 0) {
      for (let j = i; j < bytes.length; j++) {
        if (bytes[j] === 0) continue;
        if (posixTrailer && bytes[j] === 32) continue;
        return null;
      }
      return saw ? value : 0;
    }
    if (c < 48 || c > 55) return null;
    saw = true;
    value = value * 8 + (c - 48);
  }
  return saw ? value : 0;
}
function fieldName(header) {
  let end = 0;
  while (end < 100 && header[end] !== 0) end++;
  for (let j = end; j < 100; j++) if (header[j] !== 0) return null;
  let name = "";
  for (let i = 0; i < end; i++) name += String.fromCharCode(header[i]);
  return name;
}
function prefixLength(header) {
  let end = 0;
  while (end < 155 && header[345 + end] !== 0) end++;
  return end;
}
function dangerous(name) {
  if (name.charAt(0) === "/" || name.charAt(0) === "\\") return true;
  const parts = name.split("/");
  for (let i = 0; i < parts.length; i++) if (parts[i] === "..") return true;
  return false;
}
const members = [];
let fault = 0;
let faultReason = "";
function faultAt(code, reason) {
  if (fault === 0) {
    fault = code;
    faultReason = reason;
  }
}
let off = 0;
let sawEnd = false;
while (fault === 0 && off < buf.length) {
  const h = buf.subarray(off, off + 512);
  let zero = true;
  for (let i = 0; i < 512; i++) if (h[i] !== 0) { zero = false; break; }
  if (zero) { sawEnd = true; break; }
  let sum = 0;
  for (let i = 0; i < 512; i++) sum += (i >= 148 && i < 156) ? 32 : h[i];
  const recorded = plainOctal(h, 148, 8, true);
  if (recorded !== sum) { faultAt(2, ""); break; }
  const uname = fieldName(h);
  if (uname === null || uname.length === 0 || dangerous(uname) || prefixLength(h) !== 0) {
    faultAt(1, "special");
    break;
  }
  const typ = h[156];
  const size = plainOctal(h, 124, 12, false);
  if (size === null) { faultAt(2, ""); break; }
  const slash = uname.charAt(uname.length - 1) === "/";
  const directoryLike = typ === 53 || ((typ === 0 || typ === 48) && slash);
  const perm = plainOctal(h, 100, 8, false);
  if (perm === null) { faultAt(2, ""); break; }
  if (directoryLike) {
    if (size !== 0) { faultAt(1, "special"); break; }
    members.push({ name: uname, dir: true, perm: perm });
    off += 512;
    continue;
  }
  if (!(typ === 0 || typ === 48)) { faultAt(1, "special"); break; }
  const blocks = Math.ceil(size / 512);
  const dataOff = off + 512;
  const bytes = Buffer.from(buf.subarray(dataOff, dataOff + size));
  members.push({ name: uname, dir: false, perm: perm, bytes: bytes });
  off += 512 * (1 + blocks);
}
if (fault === 0 && !sawEnd) faultAt(2, "");
if (fault !== 0) finish(fault, faultReason);
if (mode !== "stage") process.exit(0);
const slashAt = wantName.lastIndexOf("/");
const root = slashAt < 0 ? "" : wantName.slice(0, slashAt);
const binary = members.filter((item) => !item.dir && item.name === wantName);
const allowedFile = {};
allowedFile[wantName] = true;
allowedFile[root + "/LICENSE"] = true;
allowedFile[root + "/README.md"] = true;
let reason = "";
if (binary.length !== 1) {
  const files = members.filter((item) => !item.dir);
  if (files.length === 1 && files[0].name === root) reason = "notdir";
  else if (files.length > 0 && files.every((item) => {
    const ix = item.name.indexOf("/");
    return ix > 0 && item.name.slice(0, ix) !== root;
  })) reason = "mismatch";
  else reason = "binary";
} else {
  for (let i = 0; i < members.length; i++) {
    const item = members[i];
    if (item.dir) {
      let bare = item.name;
      while (bare.charAt(bare.length - 1) === "/") bare = bare.slice(0, -1);
      if (bare !== root) reason = "root";
    } else if (!allowedFile[item.name]) reason = "root";
  }
}
if (reason) finish(1, reason);
if (mode !== "stage") process.exit(0);
if (fileHash !== pin) {
  process.stdout.write(fileHash + "\n");
  process.exit(3);
}
const dest = execFileSync(mktempBin, ["-d", path.join(parentDir, "aw.XXXXXX")], { encoding: "utf8" }).trim();
const destStat = fs.lstatSync(dest);
if (!destStat.isDirectory() || destStat.isSymbolicLink()) finish(2, "");
const rootDir = path.join(dest, root);
fs.mkdirSync(rootDir);
const rootParent = fs.realpathSync(dest);
const rootReal = fs.realpathSync(rootDir);
if (!rootReal.startsWith(rootParent + path.sep)) finish(1, "special");
let dirPerm = 493;
for (let i = 0; i < members.length; i++) if (members[i].dir) dirPerm = members[i].perm & 511;
fs.chmodSync(rootDir, dirPerm);
const outName = wantName.slice(slashAt + 1);
const out = path.join(rootDir, outName);
const flags = fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW;
const filePerm = binary[0].perm & 511;
const fd = fs.openSync(out, flags, filePerm);
fs.writeSync(fd, binary[0].bytes);
fs.fchmodSync(fd, filePerm);
fs.closeSync(fd);
const written = fs.lstatSync(out);
if (!written.isFile() || written.isSymbolicLink() || written.nlink !== 1) finish(1, "special");
const memberHash = crypto.createHash("sha256").update(binary[0].bytes).digest("hex");
process.stdout.write(fileHash + "\n" + memberHash + "\n" + dest + "\n");
process.exit(0);' "$@"
}

archive_plain_members() {
  read_archive "$1" members
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

if [ -n "${SCCACHE_STAGE_CHECK:-}" ]; then
  stage_status=0
  stage_parent="$("$MKTEMP_BIN" -d)" || exit 1
  read_archive - stage "${SCCACHE_STAGE_NAME:-sccache-v0.19.0-test/sccache}" "${SCCACHE_STAGE_PIN:-}" "$MKTEMP_BIN" "$stage_parent" < "$SCCACHE_STAGE_CHECK" || stage_status=$?
  if [ -z "${SCCACHE_STAGE_KEEP:-}" ]; then
    "$RM_BIN" -rf "$stage_parent" || true
  fi
  exit "$stage_status"
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
want="$(pinned_sha256 "$filename")"
captured="$tmp/hashed.bin"
shell_got="$("$TEE_BIN" "$captured" < "$tmp/pkg.tar.gz" | shell_sha256_stdin)" || fail_closed "could not hash the archive"
if [ "${#want}" -ne 64 ] || [ "$shell_got" != "$want" ]; then
  trap - ERR
  builtin printf 'sccache checksum mismatch for %s\n' "$filename" >&2
  exit 1
fi
stage_status=0
stage_out="$(read_archive - stage "${release_root}/${binary_name}" "$want" "$MKTEMP_BIN" "$tmp" < "$captured")" || stage_status=$?
got="${stage_out%%$'\n'*}"
rest="${stage_out#*$'\n'}"
member_sum=""
extract=""
if [ "$stage_out" != "$got" ]; then
  member_sum="${rest%%$'\n'*}"
  if [ "$rest" != "$member_sum" ]; then
    extract="${rest#*$'\n'}"
    extract="${extract%%$'\n'*}"
  fi
fi
if [ "${#want}" -ne 64 ] || { [ "${#got}" -eq 64 ] && [ "$got" != "$want" ]; }; then
  trap - ERR
  builtin printf 'sccache checksum mismatch for %s\n' "$filename" >&2
  exit 1
fi
if [ "$stage_status" -eq 1 ]; then
  case "$member_sum" in
    notdir)
      fail_closed "archive release root is not a directory"
      ;;
    mismatch)
      fail_closed "archive release root does not match the asset"
      ;;
    binary)
      _aw_reason="archive has no sccache binary"
      fail_closed "$_aw_reason"
      ;;
    root)
      fail_closed "archive does not contain exactly one release root"
      ;;
    *)
      fail_closed "archive has a symlink or non-regular entry"
      ;;
  esac
fi
if [ "$stage_status" -ne 0 ] || [ "${#got}" -ne 64 ] || [ -z "$extract" ]; then
  fail_closed "could not extract the archive"
fi

if ! plain_tree "$extract"; then
  fail_closed "archive has a symlink or non-regular entry"
fi
top_name="$("$NODE_BIN" -e 'const fs=require("fs"); const names=fs.readdirSync(process.argv[1]); if (names.length!==1) process.exit(1); process.stdout.write(names[0]);' "$extract")" || fail_closed "archive does not contain exactly one release root"
if [ "$top_name" != "$release_root" ]; then
  fail_closed "archive release root does not match the asset"
fi
inner="$extract/$release_root"
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
dst_sum="$(shell_sha256 "${dest}/${binary_name}")" || fail_closed "could not hash the installed binary"
member_pin="$(pinned_member_sha256 "$filename")" || fail_closed "tool cache is missing the sccache binary"
if [ "$dst_sum" != "$member_pin" ]; then
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
