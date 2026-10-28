#!/usr/bin/env bash
# Print recorded GPU skips. When AW_GPU_SKIP_IF_UNAVAILABLE is set, a log that
# contains some other line does not pass: every name in
# gpu-skip-required-macos.txt must have its own SKIP(gpu) line.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
outcome="${NEXTEST_OUTCOME:-success}"
log="${AW_GPU_SKIP_LOG:-${root}/target/gpu-skip-reasons.txt}"
required="${root}/scripts/ci/gpu-skip-required-macos.txt"

if [[ "$outcome" != "success" ]]; then
  echo "::error title=GPU tests failed::Nextest failed. A skip log does not cover a panic or a failed GPU test."
  exit 1
fi

print_log() {
  if [[ ! -s "$log" ]]; then
    return 0
  fi
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    {
      echo "## GPU tests skipped"
      echo
      echo "Each line was recorded by that test. A missing required name fails this step."
      echo
    } >>"$GITHUB_STEP_SUMMARY"
  fi
  while IFS= read -r line || [[ -n "$line" ]]; do
    safe="${line//$'\r'/ }"
    safe="${safe//$'\n'/}"
    safe="${safe//%/}"
    echo "$safe"
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
      echo "- ${safe}" >>"$GITHUB_STEP_SUMMARY"
      echo "::warning title=GPU test skipped::${safe}"
    fi
  done <"$log"
}

if [[ -z "${AW_GPU_SKIP_IF_UNAVAILABLE+x}" ]]; then
  print_log
  exit 0
fi

if [[ ! -s "$log" ]]; then
  echo "::error title=GPU skip log empty::AW_GPU_SKIP_IF_UNAVAILABLE is set and ${log} has no skip line."
  exit 1
fi

missing=0
while IFS= read -r name || [[ -n "$name" ]]; do
  [[ -z "$name" || "$name" == \#* ]] && continue
  if ! grep -E "(^|[[:space:]:])${name}:" "$log" >/dev/null; then
    echo "::error title=GPU skip missing::${name} has no SKIP(gpu) line. Another test's line does not cover it."
    missing=1
  fi
done <"$required"

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

print_log
