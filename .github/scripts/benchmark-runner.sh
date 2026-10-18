#!/bin/bash
# AstraWeave Benchmark Runner Script
# Runs benchmarks with proper JSON output for github-action-benchmark integration

set -euo pipefail

# Optional deterministic sharding. Package discovery remains authoritative; the
# selected shard is the subset whose zero-based package index matches modulo N.
SHARD_INDEX=0
SHARD_COUNT=1

while [ "$#" -gt 0 ]; do
    case "$1" in
        --shard-index)
            SHARD_INDEX="${2:-}"
            shift 2
            ;;
        --shard-count)
            SHARD_COUNT="${2:-}"
            shift 2
            ;;
        *)
            echo "[ERROR] Unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

if ! [[ "$SHARD_INDEX" =~ ^[0-9]+$ && "$SHARD_COUNT" =~ ^[1-9][0-9]*$ ]]; then
    echo "[ERROR] Shard index must be non-negative and shard count must be positive" >&2
    exit 2
fi

if [ "$SHARD_INDEX" -ge "$SHARD_COUNT" ]; then
    echo "[ERROR] Shard index $SHARD_INDEX is outside shard count $SHARD_COUNT" >&2
    exit 2
fi

# Configuration
# Week 3 Action 11: Updated to include all Week 2-3 benchmark packages
# Week 3 Action 12: Added astraweave-physics (raycast, character controller, rigid body benchmarks)
BENCHMARK_PACKAGES_STATIC=(
    astraweave-core
    astraweave-input
    astraweave-ai
    astraweave-behavior
    astraweave-stress-test
    astraweave-terrain
    astraweave-physics
)

# Production grouping. astraweave-ai and astraweave-render each have their own
# shard: on the 2026-10-14 main run their compiles were killed by the 600s
# per-package timeout before Criterion wrote an estimate, and they shared a
# shard with other multi-minute packages. Discovery remains authoritative and
# the coverage validation below fails if this map ever omits, duplicates, or
# invents a benchmark package.
BENCHMARK_SHARD_GROUPS=(
    # localsim (#330) is a 32-region CPU tick. Shard 0 is the short group, so
    # the measured map covers the crate without a fifteenth job or a longer cap.
    "astraweave-net astraweave-profiling astraweave-water astraweave-localsim"
    "astraweave-cinematics astraweave-quests astraweave-context astraweave-terrain"
    "astraweave-dialogue astraweave-llm astraweave-steam astraweave-embeddings"
    "astraweave-ecs astraweave-math astraweave-ipc astraweave-memory astraweave-destruction"
    "astraweave-fluids astraweave-scripting astraweave-author astraweave-core"
    "astraweave-net-ecs astraweave-asset-pipeline astraweave-prompts astraweave-audio"
    "astraweave-persistence-ecs astraweave-weaving astraweave-observability astraweave-input"
    "astraweave-rag astraweave-llm-eval astraweave-secrets astraweave-physics"
    "astraweave-npc astraweave-materials astraweave-gameplay astraweave-combat"
    "astraweave-scene astraweave-optimization astraweave-director astraweave-nav"
    "astraweave-sdk astraweave-coordination astraweave-persona astraweave-behavior"
    "astraweave-ui astraweave-pcg astraweave-security astraweave-stress-test"
    "astraweave-ai"
    # astraweave-render is one package and two shards. @a and @b are
    # alphabetical halves of its bench targets (see select_bench_targets).
    # Shard 13 on ceff979a (run 39089221901) and on #374 (run 39091616710)
    # was cancelled at the 60-minute job limit after two targets each used
    # the full 600s. The halves keep every target. The 600s cap stays.
    "astraweave-render@a"
    "astraweave-render@b"
)

RESULTS_DIR="${BENCHMARK_RESULTS_DIR:-benchmark_results}"
SUMMARY_FILE="$RESULTS_DIR/summary.txt"
JSON_FILE="$RESULTS_DIR/benchmarks.json"
PACKAGE_STATUS_FILE="$RESULTS_DIR/package-status.json"
TIMINGS_FILE="$RESULTS_DIR/timings.tsv"
BENCHMARK_ENTRIES_FILE="$RESULTS_DIR/.benchmark-entries.jsonl"
PACKAGE_STATUS_ENTRIES_FILE="$RESULTS_DIR/.package-status-entries.jsonl"
VERBOSE="${VERBOSE:-false}"

# Create results directory
mkdir -p "$RESULTS_DIR"
rm -f "$BENCHMARK_ENTRIES_FILE" "$PACKAGE_STATUS_ENTRIES_FILE"

# Collection exceptions must be explicit, named, and owned. The 2026-07-24
# diagnosis found no non-Criterion harnesses, so this registry is intentionally
# empty. A successful package that yields zero Criterion estimates is fatal
# unless it is added here with both a reason and an owner.
declare -A COLLECTION_EXCEPTION_REASONS=()
declare -A COLLECTION_EXCEPTION_OWNERS=()

# Logging functions
log_info() {
    echo "[INFO] $*" | tee -a "$SUMMARY_FILE"
}

log_error() {
    echo "[ERROR] $*" | tee -a "$SUMMARY_FILE" >&2
}

log_success() {
    echo "[SUCCESS] $*" | tee -a "$SUMMARY_FILE"
}

# Initialize durable report files before discovery so the selected shard line
# is retained in the uploaded summary.
{
    echo "=== AstraWeave Performance Benchmarks ==="
    echo "Date: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo "Commit: ${GITHUB_SHA:-$(git rev-parse HEAD 2>/dev/null || echo 'unknown')}"
    echo "Runner: ${RUNNER_OS:-$(uname -s)} ${RUNNER_ARCH:-$(uname -m)}"
    echo ""
} > "$SUMMARY_FILE"

printf 'package\telapsed_seconds\texecution_status\tcollection_status\tbenchmark_count\n' > "$TIMINGS_FILE"

# Auto-discover packages with benchmarks
BENCHMARK_PACKAGES=()

# Function to discover benchmark packages
discover_benchmark_packages() {
    local discovered_packages=()
    
    # Start with static list
    for pkg in "${BENCHMARK_PACKAGES_STATIC[@]}"; do
        if [ -d "$pkg/benches" ]; then
            discovered_packages+=("$pkg")
        else
            echo "[WARN] Static package $pkg has no benchmarks directory"
        fi
    done
    
    # Auto-discover additional packages with benchmarks
    for pkg_dir in */; do
        pkg_name=$(basename "$pkg_dir")
        if [ -d "$pkg_dir/benches" ] && [[ " ${BENCHMARK_PACKAGES_STATIC[*]} " != *" ${pkg_name} "* ]]; then
            # Check if this is a Rust package with benchmarks
            if [ -f "$pkg_dir/Cargo.toml" ] && grep -q "\[\[bench\]\]" "$pkg_dir/Cargo.toml"; then
                discovered_packages+=("$pkg_name")
                echo "[INFO] Auto-discovered benchmark package: $pkg_name"
            fi
        fi
    done
    
    # Set the global array
    BENCHMARK_PACKAGES=("${discovered_packages[@]}")

    if [ "$SHARD_COUNT" -eq "${#BENCHMARK_SHARD_GROUPS[@]}" ]; then
        local -A discovered_lookup=()
        local -A assigned_lookup=()
        local configured_pkg
        local discovered_pkg
        local shard_packages
        local mapping_error=0
        local selected_packages=()

        for discovered_pkg in "${BENCHMARK_PACKAGES[@]}"; do
            discovered_lookup["$discovered_pkg"]=1
        done

        for shard_packages in "${BENCHMARK_SHARD_GROUPS[@]}"; do
            for configured_pkg in $shard_packages; do
                local pkg_base="${configured_pkg%@*}"
                if [[ ! -v "discovered_lookup[$pkg_base]" ]]; then
                    log_error "Measured shard map contains undiscovered package: $configured_pkg"
                    mapping_error=1
                elif [[ -v "assigned_lookup[$configured_pkg]" ]]; then
                    log_error "Measured shard map duplicates package: $configured_pkg"
                    mapping_error=1
                else
                    assigned_lookup["$configured_pkg"]=1
                    assigned_lookup["$pkg_base"]=1
                fi
            done
        done

        for discovered_pkg in "${BENCHMARK_PACKAGES[@]}"; do
            if [[ ! -v "assigned_lookup[$discovered_pkg]" ]]; then
                log_error "Measured shard map omits discovered package: $discovered_pkg"
                mapping_error=1
            fi
        done
        if [[ -v "assigned_lookup[astraweave-render@a]" && ! -v "assigned_lookup[astraweave-render@b]" ]] \
            || [[ -v "assigned_lookup[astraweave-render@b]" && ! -v "assigned_lookup[astraweave-render@a]" ]]; then
            log_error "astraweave-render must be listed as both @a and @b"
            mapping_error=1
        fi

        if [ "$mapping_error" -ne 0 ]; then
            return 1
        fi

        read -r -a selected_packages \
            <<< "${BENCHMARK_SHARD_GROUPS[$SHARD_INDEX]}"
        BENCHMARK_PACKAGES=("${selected_packages[@]}")
    elif [ "$SHARD_COUNT" -gt 1 ]; then
        # Preserve the generic modulo mode for ad-hoc shard counts. The two
        # production workflows use the validated measured shard map above.
        local selected_packages=()
        local package_index
        for package_index in "${!BENCHMARK_PACKAGES[@]}"; do
            if [ $((package_index % SHARD_COUNT)) -eq "$SHARD_INDEX" ]; then
                selected_packages+=("${BENCHMARK_PACKAGES[$package_index]}")
            fi
        done
        BENCHMARK_PACKAGES=("${selected_packages[@]}")
    fi
    
    if [ ${#BENCHMARK_PACKAGES[@]} -eq 0 ]; then
        log_info "No benchmark packages found"
        return 1
    fi
    
    log_info "Selected shard $SHARD_INDEX/$SHARD_COUNT with ${#BENCHMARK_PACKAGES[@]} benchmark packages: ${BENCHMARK_PACKAGES[*]}"
    return 0
}

# Discover benchmark packages
if ! discover_benchmark_packages; then
    echo "[ERROR] No benchmark packages found!" | tee -a "$SUMMARY_FILE"
    exit 1
fi

BENCHMARK_COUNT=0
SUCCESS_COUNT=0
COLLECTED_PACKAGE_COUNT=0
EXCEPTION_COUNT=0
EXECUTION_FAILURE_COUNT=0
COLLECTION_FAILURE_COUNT=0
OVERALL_FAILURE=0
PACKAGE_BENCHMARK_COUNT=0
PACKAGE_COLLECTION_ERROR_COUNT=0

# Function to format time units
format_time() {
    local ns=$1
    if (( $(echo "$ns < 1000" | bc -l) )); then
        printf "%.2f ns" "$ns"
    elif (( $(echo "$ns < 1000000" | bc -l) )); then
        printf "%.2f µs" "$(echo "scale=2; $ns / 1000" | bc -l)"
    elif (( $(echo "$ns < 1000000000" | bc -l) )); then
        printf "%.2f ms" "$(echo "scale=2; $ns / 1000000" | bc -l)"
    else
        printf "%.2f s" "$(echo "scale=2; $ns / 1000000000" | bc -l)"
    fi
}

# Function to process benchmark results for a specific package
process_benchmarks() {
    local pkg=$1
    local estimates_file
    local relative_path
    local bench_name
    local mean_ns
    local formatted_time
    local -a estimates_files=()

    PACKAGE_BENCHMARK_COUNT=0
    PACKAGE_COLLECTION_ERROR_COUNT=0

    if [ ! -d "target/criterion" ]; then
        log_error "Criterion target directory not found after running $pkg"
        return 0
    fi

    # Criterion stores grouped benchmarks recursively:
    # target/criterion/<group>/<function>/new/estimates.json. Some group names
    # contain slashes and add further levels, so a one-level glob is incomplete.
    mapfile -d '' estimates_files < <(
        find target/criterion -type f -path '*/new/estimates.json' -print0 |
            sort -z
    )

    for estimates_file in "${estimates_files[@]}"; do
        relative_path=${estimates_file#target/criterion/}
        bench_name=${relative_path%/new/estimates.json}

        # Safely extract mean value with error handling.
        if mean_ns=$(jq -r '.mean.point_estimate // empty' "$estimates_file" 2>/dev/null) &&
            [ -n "$mean_ns" ] && [ "$mean_ns" != "null" ]; then
            if [[ "$mean_ns" =~ ^[0-9]+\.?[0-9]*$ ]] &&
                (( $(echo "$mean_ns > 0" | bc -l) )); then
                jq -cn \
                    --arg name "${pkg}::${bench_name}" \
                    --argjson value "$mean_ns" \
                    '{name: $name, unit: "ns", value: $value}' \
                    >> "$BENCHMARK_ENTRIES_FILE"

                BENCHMARK_COUNT=$((BENCHMARK_COUNT + 1))
                PACKAGE_BENCHMARK_COUNT=$((PACKAGE_BENCHMARK_COUNT + 1))

                formatted_time=$(format_time "$mean_ns")
                printf "  %-70s %s\n" "$bench_name" "$formatted_time" |
                    tee -a "$SUMMARY_FILE"
            else
                log_error "Invalid benchmark value for $pkg::$bench_name: $mean_ns"
                PACKAGE_COLLECTION_ERROR_COUNT=$((PACKAGE_COLLECTION_ERROR_COUNT + 1))
            fi
        else
            log_error "Could not extract valid benchmark data for $pkg::$bench_name"
            PACKAGE_COLLECTION_ERROR_COUNT=$((PACKAGE_COLLECTION_ERROR_COUNT + 1))
        fi
    done

    log_info "Collected $PACKAGE_BENCHMARK_COUNT Criterion result(s) for $pkg"
    return 0
}

# Main benchmark execution
log_info "Starting benchmark execution..."

# Bench target names come from cargo metadata. Targets with required-features
# outside the package default set are skipped, matching `cargo bench --benches`.
METADATA_JSON=""
if METADATA_JSON=$(cargo metadata --format-version 1 --no-deps 2>"${RESULTS_DIR}/metadata.err"); then
    log_info "Resolved bench targets from cargo metadata"
else
    log_error "cargo metadata failed; each package will run as one --benches invocation"
    METADATA_JSON=""
fi

bench_target_names() {
    local pkg="$1"
    if [ -z "$METADATA_JSON" ]; then
        return 0
    fi
    jq -r --arg pkg "$pkg" '
        .packages[] | select(.name == $pkg) as $p
        | ($p.features.default // []) as $defaults
        | $p.targets[]
        | select(any(.kind[]; . == "bench"))
        | select(
            ((.["required-features"] // []) | length) == 0
            or (
                (.["required-features"] // [])
                | all(. as $f | $defaults | index($f) != null)
            )
          )
        | .name
    ' <<<"$METADATA_JSON"
}

# astraweave-render@a / @b: alphabetical halves. A new bench target
# lands in one half automatically. Neither half drops a target.
select_bench_targets() {
    local pkg_token="$1"
    local pkg_base="${pkg_token%@*}"
    local half=""
    if [[ "$pkg_token" == *@* ]]; then
        half="${pkg_token#*@}"
    fi
    local -a all_targets=()
    mapfile -t all_targets < <(bench_target_names "$pkg_base" | sort)
    if [ -z "$half" ]; then
        printf '%s\n' "${all_targets[@]}"
        return
    fi
    if [ "$half" != "a" ] && [ "$half" != "b" ]; then
        log_error "Unknown bench half '$half' on $pkg_token"
        return 1
    fi
    local total=${#all_targets[@]}
    local mid=$(((total + 1) / 2))
    local -a chosen=()
    if [ "$half" = "a" ]; then
        chosen=("${all_targets[@]:0:mid}")
    else
        chosen=("${all_targets[@]:mid}")
    fi
    printf '%s\n' "${chosen[@]}"
}

for pkg in "${BENCHMARK_PACKAGES[@]}"; do
    pkg_base="${pkg%@*}"
    if [ -d "$pkg_base/benches" ]; then
        log_info "Running benchmarks for $pkg..."

        # Clear previous criterion results to avoid cross-contamination
        if [ -d "target/criterion" ]; then
            rm -rf target/criterion/* 2>/dev/null || true
        fi

        package_start=$(date +%s)
        stdout_log="${RESULTS_DIR}/${pkg}_stdout.log"
        stderr_log="${RESULTS_DIR}/${pkg}_stderr.log"
        : > "$stdout_log"
        : > "$stderr_log"

        # Compilation is not part of the 600s measurement budget. Cold
        # astraweave-ai / astraweave-render compiles were being killed before
        # Criterion wrote a single estimate. The job timeout still bounds a
        # hung rustc. Each bench target then gets its own 600s so one crate
        # with many targets is split instead of raising the limit.
        set +e
        cargo bench -p "$pkg_base" --benches --no-run \
            >> "$stdout_log" \
            2>> "$stderr_log"
        compile_exit=$?
        set -e

        execution_exit=0
        execution_status="success"
        if [ "$compile_exit" -ne 0 ]; then
            execution_exit=$compile_exit
            execution_status="failed"
            EXECUTION_FAILURE_COUNT=$((EXECUTION_FAILURE_COUNT + 1))
            OVERALL_FAILURE=1
            log_error "Benchmark compile failed for $pkg with exit $compile_exit"
        else
            mapfile -t bench_targets < <(select_bench_targets "$pkg")
            log_info "Selected ${#bench_targets[@]} bench targets for $pkg: ${bench_targets[*]}"
            if [ "${#bench_targets[@]}" -eq 0 ]; then
                if [[ "$pkg" == *@* ]]; then
                    log_error "Bench half $pkg selected no targets"
                    execution_exit=1
                    execution_status="failed"
                    EXECUTION_FAILURE_COUNT=$((EXECUTION_FAILURE_COUNT + 1))
                    OVERALL_FAILURE=1
                    bench_targets=()
                else
                    bench_targets=("__all__")
                fi
            fi
            for target in "${bench_targets[@]}"; do
                set +e
                if [ "$target" = "__all__" ]; then
                    timeout 600 cargo bench -p "$pkg_base" --benches \
                        >> "$stdout_log" 2>> "$stderr_log"
                else
                    timeout 600 cargo bench -p "$pkg_base" --bench "$target" \
                        >> "$stdout_log" 2>> "$stderr_log"
                fi
                one_exit=$?
                set -e
                if [ "$one_exit" -eq 0 ]; then
                    log_info "Benchmark target $pkg::$target completed"
                elif [ "$one_exit" -eq 124 ]; then
                    execution_exit=124
                    execution_status="timeout"
                    log_error "Benchmark target $pkg::$target timed out after 600s"
                else
                    if [ "$execution_status" != "timeout" ]; then
                        execution_exit=$one_exit
                        execution_status="failed"
                    fi
                    log_error "Benchmark target $pkg::$target failed with exit $one_exit"
                fi
            done
            if [ "$execution_status" != "success" ]; then
                EXECUTION_FAILURE_COUNT=$((EXECUTION_FAILURE_COUNT + 1))
                OVERALL_FAILURE=1
            fi
        fi

        package_end=$(date +%s)
        elapsed_seconds=$((package_end - package_start))

        if [ "$execution_status" = "success" ]; then
            log_success "Benchmark execution completed for $pkg in ${elapsed_seconds}s"
        elif [ "$execution_status" = "timeout" ]; then
            log_error "Benchmark execution timed out for $pkg after ${elapsed_seconds}s"
        else
            log_error "Benchmark execution failed for $pkg with exit $execution_exit after ${elapsed_seconds}s"
        fi

        # Preserve all Criterion output produced before either success or
        # failure. Partial results remain useful, but never erase a red status.
        process_benchmarks "$pkg"

        collection_status="unavailable"
        exception_reason=""
        exception_owner=""
        if [ "$PACKAGE_COLLECTION_ERROR_COUNT" -gt 0 ]; then
            collection_status="invalid"
            COLLECTION_FAILURE_COUNT=$((COLLECTION_FAILURE_COUNT + 1))
            OVERALL_FAILURE=1
            log_error "Collection encountered $PACKAGE_COLLECTION_ERROR_COUNT invalid Criterion result(s) for $pkg"
        elif [ "$PACKAGE_BENCHMARK_COUNT" -gt 0 ]; then
            collection_status="collected"
            COLLECTED_PACKAGE_COUNT=$((COLLECTED_PACKAGE_COUNT + 1))
        elif [ "$execution_status" = "success" ]; then
            if [[ -v "COLLECTION_EXCEPTION_REASONS[$pkg]" ]] &&
                [[ -v "COLLECTION_EXCEPTION_OWNERS[$pkg]" ]]; then
                collection_status="exception"
                exception_reason=${COLLECTION_EXCEPTION_REASONS[$pkg]}
                exception_owner=${COLLECTION_EXCEPTION_OWNERS[$pkg]}
                EXCEPTION_COUNT=$((EXCEPTION_COUNT + 1))
                log_info "Named collection exception for $pkg: $exception_reason (owner: $exception_owner)"
            else
                collection_status="missing"
                COLLECTION_FAILURE_COUNT=$((COLLECTION_FAILURE_COUNT + 1))
                OVERALL_FAILURE=1
                log_error "Successful benchmark execution produced zero collectible Criterion results for $pkg"
            fi
        fi

        if [ "$execution_status" = "success" ] &&
            { [ "$collection_status" = "collected" ] || [ "$collection_status" = "exception" ]; }; then
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        fi

        jq -cn \
            --arg package "$pkg" \
            --arg execution_status "$execution_status" \
            --argjson execution_exit "$execution_exit" \
            --arg collection_status "$collection_status" \
            --argjson benchmark_count "$PACKAGE_BENCHMARK_COUNT" \
            --argjson elapsed_seconds "$elapsed_seconds" \
            --arg exception_reason "$exception_reason" \
            --arg exception_owner "$exception_owner" \
            '{
                package: $package,
                execution_status: $execution_status,
                execution_exit: $execution_exit,
                collection_status: $collection_status,
                benchmark_count: $benchmark_count,
                elapsed_seconds: $elapsed_seconds,
                exception_reason: (if $exception_reason == "" then null else $exception_reason end),
                exception_owner: (if $exception_owner == "" then null else $exception_owner end)
            }' >> "$PACKAGE_STATUS_ENTRIES_FILE"

        printf '%s\t%s\t%s\t%s\t%s\n' \
            "$pkg" \
            "$elapsed_seconds" \
            "$execution_status" \
            "$collection_status" \
            "$PACKAGE_BENCHMARK_COUNT" \
            >> "$TIMINGS_FILE"

        if [ "$execution_status" != "success" ]; then
            {
                echo "--- $pkg stderr ---"
                tail -n 200 "${RESULTS_DIR}/${pkg}_stderr.log"
                echo "--- end stderr ---"
            } >> "$SUMMARY_FILE"
        fi

        if [ "$VERBOSE" = "true" ]; then
            {
                echo "--- $pkg stdout ---"
                tail -n 20 "${RESULTS_DIR}/${pkg}_stdout.log"
                echo "--- end stdout ---"
            } >> "$SUMMARY_FILE"
        fi

        echo "" >> "$SUMMARY_FILE"
    else
        # Discovery selected this package only because its benches directory
        # existed. Losing it mid-run is therefore an internal consistency error.
        OVERALL_FAILURE=1
        EXECUTION_FAILURE_COUNT=$((EXECUTION_FAILURE_COUNT + 1))
        log_error "Selected package has no benchmarks directory at execution time: $pkg"
    fi
done

# Materialize deterministic JSON arrays from the per-record JSONL files.
if [ -s "$BENCHMARK_ENTRIES_FILE" ]; then
    jq -s '.' "$BENCHMARK_ENTRIES_FILE" > "$JSON_FILE"
else
    echo '[]' > "$JSON_FILE"
fi

if [ -s "$PACKAGE_STATUS_ENTRIES_FILE" ]; then
    jq -s '.' "$PACKAGE_STATUS_ENTRIES_FILE" > "$PACKAGE_STATUS_FILE"
else
    echo '[]' > "$PACKAGE_STATUS_FILE"
fi

rm -f "$BENCHMARK_ENTRIES_FILE" "$PACKAGE_STATUS_ENTRIES_FILE"

# Generate final summary
{
    echo "=== Execution Summary ==="
    echo "Total packages processed: ${#BENCHMARK_PACKAGES[@]}"
    echo "Packages with successful benchmarks: $SUCCESS_COUNT"
    echo "Packages with collected Criterion output: $COLLECTED_PACKAGE_COUNT"
    echo "Named collection exceptions: $EXCEPTION_COUNT"
    echo "Execution failures: $EXECUTION_FAILURE_COUNT"
    echo "Collection failures: $COLLECTION_FAILURE_COUNT"
    echo "Total benchmarks collected: $BENCHMARK_COUNT"
    echo ""
    echo "Results saved to:"
    echo "  - Summary: $SUMMARY_FILE"
    echo "  - JSON data: $JSON_FILE"
    echo "  - Package status: $PACKAGE_STATUS_FILE"
    echo "  - Timings: $TIMINGS_FILE"
} | tee -a "$SUMMARY_FILE"

# Validate JSON outputs.
if jq empty "$JSON_FILE" "$PACKAGE_STATUS_FILE" 2>/dev/null; then
    log_success "Generated valid benchmark and package-status JSON"
else
    log_error "Generated benchmark or package-status JSON is invalid"
    exit 1
fi

status_count=$(jq 'length' "$PACKAGE_STATUS_FILE")
if [ "$status_count" -ne "${#BENCHMARK_PACKAGES[@]}" ]; then
    log_error "Package status count $status_count does not match selected package count ${#BENCHMARK_PACKAGES[@]}"
    OVERALL_FAILURE=1
fi

# Display final results
echo ""
echo "=== Benchmark Results Summary ==="
cat "$SUMMARY_FILE"

# Fail honestly after all package results, failure logs, statuses, and timings
# have been written. A partial collection never converts a red package to green.
if [ "$OVERALL_FAILURE" -ne 0 ]; then
    log_error "One or more benchmark packages failed execution or collection"
    exit 1
fi

if [ $((COLLECTED_PACKAGE_COUNT + EXCEPTION_COUNT)) -ne "${#BENCHMARK_PACKAGES[@]}" ]; then
    log_error "Successful shard did not account for every selected package"
    exit 1
fi

log_success "All selected benchmark packages executed and were accounted for"
exit 0
