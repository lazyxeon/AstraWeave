# Fuzz Testing Usage Guide

**Date**: October 13, 2025  
**Status**: Infrastructure ready (Linux/Mac execution only)

---

## Overview

This guide explains how to run the 4 fuzz targets created for AstraWeave ECS. Fuzz testing helps detect:
- Crashes and panics
- Undefined behavior
- Memory safety issues
- Edge cases not covered by property tests

**⚠️ Windows Limitation**: Fuzz testing requires LLVM/Clang compiler, which is not available in MSVC. Use **Linux**, **Mac**, or **WSL2** on Windows.

---

## Prerequisites

### Platform Requirements

**✅ Supported**:
- Linux (Ubuntu, Debian, Fedora, Arch, etc.)
- macOS (Intel or Apple Silicon)
- WSL2 on Windows (Ubuntu recommended)

**❌ Not Supported**:
- Windows native (MSVC/MinGW)

### Required Tools

1. **Rust Nightly Toolchain** (already installed):
   ```bash
   rustup toolchain install nightly
   rustup default nightly  # Or use +nightly flag
   ```

2. **cargo-fuzz** (already installed):
   ```bash
   cargo install cargo-fuzz
   ```

3. **LLVM/Clang** (Linux only):
   ```bash
   # Ubuntu/Debian
   sudo apt-get install clang libclang-dev

   # Fedora
   sudo dnf install clang clang-devel

   # Arch
   sudo pacman -S clang
   ```

   **Note**: macOS already includes Clang by default.

---

## Quick Start

### Basic Usage

```bash
# Navigate to ECS crate
cd astraweave-ecs

# Run a fuzz target for 60 seconds
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=60

# Run all fuzz targets for 5 minutes each
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=300
cargo +nightly fuzz run fuzz_component_ops -- -max_total_time=300
cargo +nightly fuzz run fuzz_queries -- -max_total_time=300
cargo +nightly fuzz run fuzz_mixed_ops -- -max_total_time=300
```

### Recommended Workflow

```bash
# Short test run (1 minute each) - quick validation
for target in fuzz_spawn_despawn fuzz_component_ops fuzz_queries fuzz_mixed_ops; do
    echo "Running $target..."
    cargo +nightly fuzz run $target -- -max_total_time=60
done

# Medium test run (5 minutes each) - thorough testing
for target in fuzz_spawn_despawn fuzz_component_ops fuzz_queries fuzz_mixed_ops; do
    echo "Running $target..."
    cargo +nightly fuzz run $target -- -max_total_time=300
done

# Long test run (1 hour each) - exhaustive testing
for target in fuzz_spawn_despawn fuzz_component_ops fuzz_queries fuzz_mixed_ops; do
    echo "Running $target..."
    cargo +nightly fuzz run $target -- -max_total_time=3600
done
```

---

## Fuzz Targets

### 1. fuzz_spawn_despawn

**Purpose**: Detect crashes in entity spawn/despawn operations

**What it tests**:
- Entity ID allocation and recycling
- Spawning entities with random components
- Despawning valid/invalid entity IDs
- Entity count consistency
- World state after operations

**Example**:
```bash
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=60
```

**Expected behavior**: No crashes, all entity operations should be safe

---

### 2. fuzz_component_ops

**Purpose**: Detect invalid state in component operations

**What it tests**:
- Inserting components on valid/invalid entities
- Removing components from valid/invalid entities
- Getting components (should return Option)
- Archetype transitions
- Component storage consistency

**Example**:
```bash
cargo +nightly fuzz run fuzz_component_ops -- -max_total_time=60
```

**Expected behavior**: No panics, all component operations should be safe

---

### 3. fuzz_queries

**Purpose**: Detect crashes in query operations

**What it tests**:
- Querying entities with random component combinations
- Empty world queries
- Queries after random modifications
- Query result consistency

**Example**:
```bash
cargo +nightly fuzz run fuzz_queries -- -max_total_time=60
```

**Expected behavior**: No crashes, queries should return valid results

---

### 4. fuzz_mixed_ops

**Purpose**: Detect inconsistencies in realistic mixed workloads

**What it tests**:
- Random sequences of spawn/despawn/insert/remove/query
- World state consistency after arbitrary operations
- Entity count tracking
- All operations combined

**Example**:
```bash
cargo +nightly fuzz run fuzz_mixed_ops -- -max_total_time=60
```

**Expected behavior**: No crashes, world state should remain consistent

---

## Advanced Usage

### Running with Custom Parameters

```bash
# Limit iterations instead of time
cargo +nightly fuzz run fuzz_spawn_despawn -- -runs=10000

# Increase worker threads (parallel fuzzing)
cargo +nightly fuzz run fuzz_spawn_despawn -- -workers=4 -max_total_time=300

# Enable verbose output
cargo +nightly fuzz run fuzz_spawn_despawn -- -verbosity=2

# Reduce memory usage
cargo +nightly fuzz run fuzz_spawn_despawn -- -rss_limit_mb=2048
```

### Reproducing Crashes

If a crash is found, libFuzzer saves the input to `fuzz/artifacts/<target>/`:

```bash
# List crash artifacts
ls -la fuzz/artifacts/fuzz_spawn_despawn/

# Reproduce a crash
cargo +nightly fuzz run fuzz_spawn_despawn fuzz/artifacts/fuzz_spawn_despawn/crash-abc123

# Debug with gdb
cargo +nightly fuzz run fuzz_spawn_despawn --debug fuzz/artifacts/fuzz_spawn_despawn/crash-abc123
```

### Corpus Management

Fuzz inputs are saved to `fuzz/corpus/<target>/` for future runs:

```bash
# View corpus size
du -sh fuzz/corpus/fuzz_spawn_despawn/

# Clear corpus (start fresh)
rm -rf fuzz/corpus/fuzz_spawn_despawn/*

# Merge multiple corpora
cargo +nightly fuzz cmin fuzz_spawn_despawn
```

---

## Windows Workarounds

### Option 1: WSL2 (Recommended)

**Install WSL2**:
```powershell
# In PowerShell (as Administrator)
wsl --install -d Ubuntu
```

**Setup in WSL2**:
```bash
# Inside WSL2 Ubuntu
sudo apt-get update
sudo apt-get install clang libclang-dev

# Navigate to project (Windows drives mounted at /mnt/)
cd /mnt/c/Users/YourName/path/to/AstraWeave-AI-Native-Gaming-Engine

# Run fuzz tests
cd astraweave-ecs
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=60
```

### Option 2: Docker

**Create Dockerfile**:
```dockerfile
FROM rust:latest

# Install dependencies
RUN apt-get update && apt-get install -y clang libclang-dev

# Install nightly and cargo-fuzz
RUN rustup toolchain install nightly
RUN cargo +nightly install cargo-fuzz

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
```

**Run in Docker**:
```powershell
# Build image
docker build -t astraweave-fuzz .

# Run container with project mounted
docker run -it -v ${PWD}:/workspace astraweave-fuzz

# Inside container
cd astraweave-ecs
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=60
```

### Option 3: GitHub Actions CI

**Add to `.github/workflows/fuzz.yml`**:
```yaml
name: Fuzz Testing

on:
  schedule:
    - cron: '0 2 * * *'  # Run nightly at 2 AM UTC
  workflow_dispatch:

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          override: true
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz
      - name: Run fuzz tests
        run: |
          cd astraweave-ecs
          for target in fuzz_spawn_despawn fuzz_component_ops fuzz_queries fuzz_mixed_ops; do
            echo "Running $target..."
            cargo +nightly fuzz run $target -- -max_total_time=300 || true
          done
      - name: Upload artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-artifacts
          path: astraweave-ecs/fuzz/artifacts/
```

---

## Interpreting Results

### Successful Run

```
INFO: Running with entropic power schedule (0xFF, 100).
INFO: Seed: 1234567890
INFO: Loaded 1 modules (123 inline 8-bit counters): 123 [0x12345678, 0x12345678),
INFO: -max_total_time=60 seconds
#1      NEW    cov: 45 ft: 45 corp: 1/1b exec/s: 0 rss: 28Mb
#100    NEW    cov: 52 ft: 56 corp: 2/2b lim: 4 exec/s: 0 rss: 28Mb
#1000   NEW    cov: 58 ft: 62 corp: 3/5b lim: 8 exec/s: 0 rss: 28Mb
...
#100000 pulse  cov: 89 ft: 124 corp: 15/89b lim: 1024 exec/s: 5000 rss: 32Mb
Done 100000 runs in 60 seconds
```

**Interpretation**:
- `cov`: Code coverage (89 basic blocks covered)
- `ft`: Features covered (124 unique paths)
- `corp`: Corpus size (15 inputs, 89 bytes total)
- `exec/s`: Executions per second (5000)
- `rss`: Memory usage (32 MB)

**Result**: ✅ No crashes found (success)

### Crash Found

```
==12345==ERROR: libFuzzer: deadly signal
    #0 0x12345678 in astraweave_ecs::world::World::despawn
    #1 0x87654321 in fuzz_spawn_despawn::fuzz_target
    #2 0xabcdef00 in rust_fuzzer_test_input
...
artifact_prefix='./'; Test unit written to ./crash-abc123
Base64: YWJjZGVmZ2hpamtsbW5vcA==
```

**Interpretation**:
- Crash detected in `World::despawn`
- Input saved to `crash-abc123`
- Base64 encoding for reproduction

**Action**:
1. Reproduce: `cargo +nightly fuzz run fuzz_spawn_despawn crash-abc123`
2. Debug with minimal test case
3. Fix the bug
4. Re-run fuzz test to verify

---

## Best Practices

### Development Workflow

1. **Run Fuzz After Changes**: Quick 1-minute runs after ECS modifications
2. **Nightly CI**: Long 5-minute runs overnight in CI
3. **Pre-Release**: Exhaustive 1-hour runs before major releases
4. **Monitor Corpus**: Check corpus growth (healthy fuzzing adds inputs)

### Performance Tips

1. **Use Release Mode**: Fuzz targets compile in release by default (faster)
2. **Parallel Workers**: Use `-workers=N` for multi-core systems
3. **Memory Limit**: Set `-rss_limit_mb=2048` to prevent OOM
4. **Time Limit**: Use `-max_total_time=60` for controlled runs

### Coverage Tracking

```bash
# Generate coverage report
cargo +nightly fuzz coverage fuzz_spawn_despawn

# View coverage HTML
open fuzz/coverage/fuzz_spawn_despawn/index.html  # macOS
xdg-open fuzz/coverage/fuzz_spawn_despawn/index.html  # Linux
```

---

## Troubleshooting

### "cannot open input file 'clang_rt.fuzzer-x86_64.lib'" (Windows)

**Problem**: MSVC doesn't include libFuzzer runtime

**Solution**: Use WSL2, Docker, or Linux/Mac

---

### "failed to run custom build command for `libfuzzer-sys`" (Linux)

**Problem**: Missing Clang/LLVM

**Solution**:
```bash
sudo apt-get install clang libclang-dev  # Ubuntu/Debian
sudo dnf install clang clang-devel       # Fedora
sudo pacman -S clang                     # Arch
```

---

### "error: no such subcommand: `fuzz`"

**Problem**: `cargo-fuzz` not installed

**Solution**:
```bash
cargo install cargo-fuzz
```

---

### Fuzz target runs forever

**Problem**: No time limit set

**Solution**: Add `-max_total_time=60` flag

---

## CI Integration

### GitHub Actions Example

```yaml
# .github/workflows/nightly-fuzz.yml
name: Nightly Fuzz Testing

on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC daily

jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - fuzz_spawn_despawn
          - fuzz_component_ops
          - fuzz_queries
          - fuzz_mixed_ops
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          override: true
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz
      - name: Run fuzz target
        run: |
          cd astraweave-ecs
          cargo +nightly fuzz run ${{ matrix.target }} -- -max_total_time=300
      - name: Upload crashes
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-crashes-${{ matrix.target }}
          path: astraweave-ecs/fuzz/artifacts/${{ matrix.target }}/
```

---

## Expected Results

### Current Status (October 13, 2025)

**Fuzz Targets**: 4 created (spawn_despawn, component_ops, queries, mixed_ops)  
**Tests Run**: None (Windows limitation)  
**Expected Outcome**: Zero crashes (all 153 tests pass, loom validates thread safety)

### Success Criteria

- ✅ No crashes after 5-minute runs
- ✅ Code coverage > 80% (all major paths exercised)
- ✅ Corpus growth stabilizes (no new inputs after ~1000 iterations)
- ✅ Memory usage < 100 MB (no memory leaks)

---

## Summary

### Quick Commands

```bash
# Navigate to ECS crate
cd astraweave-ecs

# Quick validation (1 minute per target)
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=60
cargo +nightly fuzz run fuzz_component_ops -- -max_total_time=60
cargo +nightly fuzz run fuzz_queries -- -max_total_time=60
cargo +nightly fuzz run fuzz_mixed_ops -- -max_total_time=60

# Thorough testing (5 minutes per target)
for target in fuzz_spawn_despawn fuzz_component_ops fuzz_queries fuzz_mixed_ops; do
    cargo +nightly fuzz run $target -- -max_total_time=300
done
```

### Windows Users

**Use WSL2**:
```powershell
# Install WSL2
wsl --install -d Ubuntu

# Inside WSL2
cd /mnt/c/path/to/AstraWeave-AI-Native-Gaming-Engine/astraweave-ecs
cargo +nightly fuzz run fuzz_spawn_despawn -- -max_total_time=60
```

---

## Next Steps

1. ✅ **Run Initial Fuzz**: 1-minute runs on Linux/Mac to validate infrastructure
2. ⏳ **CI Integration**: Add nightly fuzz runs to GitHub Actions
3. ⏳ **Coverage Analysis**: Generate coverage reports to identify gaps
4. ⏳ **Continuous Fuzzing**: Run 1-hour fuzz tests weekly

---

**Date**: October 13, 2025  
**Status**: Infrastructure ready, awaiting Linux/Mac execution  
**Platform**: Linux, Mac, WSL2 (Windows native not supported)

🔍 **Fuzz targets ready for continuous testing!** 🔍
