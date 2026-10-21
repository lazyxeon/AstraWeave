# Temperature Experiment - Quick Start Instructions

**Current Status**: Ready to execute temperature experiments  
**Baseline Validated**: Temperature 0.5 (100% success, 21.2s avg latency, 20 runs)  

---

## Quick Steps

### Test Temperature 0.3 (Deterministic)

#### 1. Modify Code
Open `examples\hello_companion\src\main.rs` line ~726 and change:
```rust
.with_temperature(0.5)        // CURRENT
```
to:
```rust
.with_temperature(0.3)        // CHANGE TO THIS
```

#### 2. Recompile
```powershell
cargo build -p hello_companion --release
```
Expected time: 15-30 seconds

#### 3. Run 10 Tests
```powershell
cd scripts
.\test_hermes2pro_validation.ps1 -Iterations 10 -OutputFile "hermes2pro_temp_0.3.csv"
```
Expected time: ~3.5 minutes

#### 4. Review Results
Check terminal output for summary statistics. Results saved to `scripts\hermes2pro_temp_0.3.csv`.

---

### Test Temperature 0.7 (Creative)

#### 1. Modify Code Again
Change line ~726 to:
```rust
.with_temperature(0.7)        // CHANGE TO THIS
```

#### 2. Recompile
```powershell
cargo build -p hello_companion --release
```

#### 3. Run 10 Tests
```powershell
.\test_hermes2pro_validation.ps1 -Iterations 10 -OutputFile "hermes2pro_temp_0.7.csv"
```

#### 4. Review Results
Results saved to `scripts\hermes2pro_temp_0.7.csv`.

---

## After Both Tests Complete

### Compare Results

You'll have 3 datasets:
- `hermes2pro_temp_0.3.csv` (10 runs @ temp 0.3)
- `hermes2pro_extended_validation.csv` (20 runs @ temp 0.5) ✅ BASELINE
- `hermes2pro_temp_0.7.csv` (10 runs @ temp 0.7)

### Key Metrics to Compare

| Metric | Temp 0.3 | Temp 0.5 | Temp 0.7 |
|--------|----------|----------|----------|
| Success Rate | ? | **100%** (20/20) | ? |
| Avg Latency | ? | **21.2s** | ? |
| Tool Diversity | ? | **8 tools** | ? |
| Fallback Rate | ? | **0%** | ? |

### Decision Criteria

**Use Temperature 0.3 if**: Highest success rate AND low latency  
**Use Temperature 0.5 if**: Balanced results (current baseline)  
**Use Temperature 0.7 if**: High success rate AND need creative/varied gameplay  

---

## Reset to Baseline

After experiments, restore baseline:
```rust
.with_temperature(0.5)        // RESTORE TO BASELINE
```

Then recompile before production use.

---

**Total Time**: ~45-60 minutes for both experiments + analysis  
**Next**: Create comparative analysis document with recommendations
