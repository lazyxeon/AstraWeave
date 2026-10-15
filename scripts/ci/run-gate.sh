#!/usr/bin/env bash
# Fast gate: fmt, one clippy (per-crate [lints]), cargo check.
# Docs-only pull requests exit before any Rust tool runs.
set -euo pipefail

docs="${AW_CI_DOCS_ONLY:-false}"
full="${AW_CI_FULL:-false}"
packages="${AW_CI_PACKAGES:-}"

if [ "$docs" = "true" ]; then
  echo "Docs-only change; rust fmt, clippy, and check are skipped"
  exit 0
fi

cargo fmt --all --check

if [ "$full" = "true" ]; then
  mapfile -t clippy_args < <(python3 scripts/ci/affected_crates.py --clippy-workspace-args)
  cargo clippy "${clippy_args[@]}"
  # AW_CI_EXCLUDES is a space-separated list of --exclude flags.
  # shellcheck disable=SC2086
  cargo check --locked --workspace ${AW_CI_EXCLUDES:-}
elif [ -n "$packages" ]; then
  mapfile -t clippy_args < <(python3 scripts/ci/affected_crates.py --clippy-packages "$packages")
  if [ "${#clippy_args[@]}" -gt 0 ]; then
    cargo clippy "${clippy_args[@]}"
  else
    echo "Affected packages have no library target; clippy --lib skipped"
  fi
  IFS=',' read -ra pkgs <<< "$packages"
  check=(cargo check --locked)
  for pkg in "${pkgs[@]}"; do
    if [ -n "$pkg" ]; then
      check+=(-p "$pkg")
    fi
  done
  "${check[@]}"
else
  echo "No crate changes; cargo fmt is the rust gate"
fi
