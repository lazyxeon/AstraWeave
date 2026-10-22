#!/usr/bin/env bash
# Install one prebuilt cargo-audit or cargo-deny binary.
# These jobs do not compile the workspace. rust-ci-setup's apt stack and the
# shared Linux-astraweave crate cache ran for about 18 minutes inside a
# 20-minute job (Security Audit 39795262676, Doc-Truth Lint 39793896957),
# and the job timeout cancelled the step before the tool ran.
set -euo pipefail

tool="${1:?tool name}"
dest="${HOME}/.local/bin"
mkdir -p "$dest"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

case "$tool" in
  cargo-deny)
    url="https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz"
    sum="9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f"
    inner="cargo-deny-0.20.2-x86_64-unknown-linux-musl/cargo-deny"
    ;;
  cargo-audit)
    url="https://github.com/rustsec/rustsec/releases/download/cargo-audit%2Fv0.22.2/cargo-audit-x86_64-unknown-linux-musl-v0.22.2.tgz"
    sum="7fb9497f8594b389e5fce5ef9b92db08432996895b2e0c5a0167a69ed445c428"
    inner="cargo-audit-x86_64-unknown-linux-musl-v0.22.2/cargo-audit"
    ;;
  *)
    echo "unknown tool: $tool" >&2
    exit 1
    ;;
esac

curl -fsSL -o "$tmp/tool.tgz" "$url"
echo "$sum  $tmp/tool.tgz" | sha256sum -c -
tar -xzf "$tmp/tool.tgz" -C "$tmp"
install -m 0755 "$tmp/$inner" "$dest/$tool"
if [ -n "${GITHUB_PATH:-}" ]; then
  echo "$dest" >> "$GITHUB_PATH"
fi
export PATH="$dest:$PATH"
"$tool" --version
