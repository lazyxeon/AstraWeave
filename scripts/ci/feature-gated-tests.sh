#!/usr/bin/env bash
# Tests the default nextest archive does not compile.
# astraweave-memory's component module is cfg(feature = "bevy").
# hello_companion's metrics demo is cfg(feature = "metrics").
set -euo pipefail

profile="${AW_CI_PROFILE:-ci-pr}"
full="${AW_CI_FULL:-false}"
pkgs=",${AW_CI_PACKAGES:-},"

ran=false
if [ "$full" = "true" ] || [[ "$pkgs" == *",astraweave-memory,"* ]]; then
  cargo nextest run --locked --profile "$profile" -p astraweave-memory --features bevy --lib
  ran=true
fi
if [ "$full" = "true" ] || [[ "$pkgs" == *",hello_companion,"* ]]; then
  cargo nextest run --locked --profile "$profile" -p hello_companion --features metrics
  ran=true
fi
if [ "$ran" = false ]; then
  echo "astraweave-memory and hello_companion are not in the affected set; skipping feature-gated tests"
fi
