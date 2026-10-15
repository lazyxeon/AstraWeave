#!/usr/bin/env bash
# cargo-nextest for the PR and nightly test jobs.
# Determinism tests are not retried; that override lives in .config/nextest.toml
# under profile.ci. Pass --retries 0 as well when the caller sets AW_CI_RETRIES=0.
set -euo pipefail

full="${AW_CI_FULL:-false}"
packages="${AW_CI_PACKAGES:-}"
partition="${AW_CI_PARTITION:-}"

if [ "$full" != "true" ] && [ -z "$packages" ]; then
  echo "No crates selected; tests skipped"
  exit 0
fi

args=(cargo nextest run --locked --profile ci)
if [ "${AW_CI_RETRIES+x}" = "x" ]; then
  args+=(--retries "$AW_CI_RETRIES")
fi

if [ "$full" = "true" ]; then
  # shellcheck disable=SC2086
  extra=(${AW_CI_EXCLUDES:-})
  args+=(--workspace)
  if [ "${#extra[@]}" -gt 0 ]; then
    args+=("${extra[@]}")
  fi
else
  IFS=',' read -ra pkgs <<< "$packages"
  for pkg in "${pkgs[@]}"; do
    if [ -n "$pkg" ]; then
      args+=(-p "$pkg")
    fi
  done
fi

if [ -n "$partition" ]; then
  args+=(--partition "$partition")
fi

printf 'Running:'
printf ' %q' "${args[@]}"
printf '\n'
"${args[@]}"
