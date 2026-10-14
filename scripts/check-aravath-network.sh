#!/usr/bin/env bash
# Fail if the Aravath greybox dependency graph links an HTTP client or tokio's
# net feature. The demo has no updater and does not open sockets.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

tree=$(cargo tree -p aravath_runtime --prefix none --format '{lib}')
banned=(reqwest ureq hyper attohttpc isahc curl curl-sys native-tls openssl openssl-sys)
for name in "${banned[@]}"; do
  if printf '%s\n' "$tree" | grep -qx "$name"; then
    echo "aravath_greybox links $name"
    exit 1
  fi
done

features=$(cargo tree -p aravath_runtime -i tokio --format '{f}')
if printf '%s\n' "$features" | tr ',' '\n' | grep -qx 'net'; then
  echo "tokio net is enabled in the aravath_greybox graph:"
  echo "$features"
  exit 1
fi
if printf '%s\n' "$tree" | grep -qx 'socket2'; then
  echo "aravath_greybox links socket2"
  exit 1
fi

echo "aravath_greybox has no HTTP client and tokio/net is off"
