#!/usr/bin/env bash
# cargo-nextest for the PR and nightly test jobs.
# Determinism tests are not retried; that override lives in .config/nextest.toml
# under profile.ci (and is inherited by profile ci-pr).
# Pass --retries 0 as well when the caller sets AW_CI_RETRIES=0.
#
# Windows and Linux pull requests, and nightly Windows and macOS Test full,
# build one archive (AW_CI_ARCHIVE_OUT). Partition jobs run it
# (AW_CI_ARCHIVE_RUN) so each partition does not compile the workspace.
set -euo pipefail

full="${AW_CI_FULL:-false}"
packages="${AW_CI_PACKAGES:-}"
partition="${AW_CI_PARTITION:-}"
profile="${AW_CI_PROFILE:-ci}"
archive_out="${AW_CI_ARCHIVE_OUT:-}"
archive_run="${AW_CI_ARCHIVE_RUN:-}"

case "$profile" in
  ci-pr|ci-pr-archive|ci-pr-linux-archive)
    echo "Profile $profile excludes scripts/ci/deferred_pr_tests.txt (issue #254, owner PR #217)."
    echo "Profile ci on main and the nightly schedule still runs those tests."
    ;;
  ci-full-archive)
    echo "Profile ci-full-archive is profile ci without the four CARGO_BIN_EXE binaries."
    echo "The Test full archive job runs those binaries on the machine that compiled them."
    ;;
esac
if [ "$profile" = "ci-pr-archive" ]; then
  echo "Archive partitions also apply scripts/ci/archive_only_skips.txt."
fi
if [ "$profile" = "ci-pr-linux-archive" ]; then
  echo "Linux archive partitions skip the binaries in scripts/ci/linux_archive_skips.txt."
  echo "The archive build job runs those binaries, where CARGO_BIN_EXE is this machine."
fi

run_nextest() {
  printf 'Running:'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

if [ -n "$archive_run" ]; then
  args=(cargo nextest run --archive-file "$archive_run" --profile "$profile")
  if [ "${AW_CI_RETRIES+x}" = "x" ]; then
    args+=(--retries "$AW_CI_RETRIES")
  fi
  if [ -n "$partition" ]; then
    args+=(--partition "$partition")
  fi
  run_nextest "${args[@]}"
  exit 0
fi

if [ "$full" != "true" ] && [ -z "$packages" ]; then
  echo "No crates selected; tests skipped"
  exit 0
fi

if [ -n "$archive_out" ]; then
  # No --profile: the archive must contain every selected test. Profile ci-pr
  # filters at run time, on the partition jobs.
  mkdir -p "$(dirname "$archive_out")"
  args=(cargo nextest archive --archive-file "$archive_out" --locked)
else
  args=(cargo nextest run --locked --profile "$profile")
  if [ "${AW_CI_RETRIES+x}" = "x" ]; then
    args+=(--retries "$AW_CI_RETRIES")
  fi
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

if [ -n "$partition" ] && [ -z "$archive_out" ]; then
  args+=(--partition "$partition")
fi

run_nextest "${args[@]}"
