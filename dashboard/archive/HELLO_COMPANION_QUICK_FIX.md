# Hello Companion - Quick Fix Guide

## 🚨 Problem
hello_companion fails with **49 compilation errors** due to API mismatches.

---

## ✅ Solution (2 Minutes)

### Step 1: Replace main.rs
```powershell
# Open: HELLO_COMPANION_FIXED.txt
# Copy ALL (Ctrl+A, Ctrl+C)

# Open: examples\hello_companion\src\main.rs  
# Paste (Ctrl+A, Ctrl+V, Ctrl+S)
```

### Step 2: Test
```powershell
cargo check -p hello_companion --features llm,ollama
```

**Expected**: ✅ Compiles successfully

### Step 3: Run with Phi-3
```powershell
# Verify Ollama running
ollama ps

# Run
cargo run -p hello_companion --release --features llm,ollama
```

---

## 🎯 What Was Fixed

| Issue | Count | Fix |
|-------|-------|-----|
| `snap.threats` → `snap.enemies` | 35 | Changed to actual WorldSnapshot API |
| BehaviorGraph builder methods | 12 | Use BehaviorNode constructors |
| Missing `plan_id` field | 5 | Added to all PlanIntent |
| `reqwest::blocking` | 1 | Use async with tokio |
| Missing Revive pattern | 1 | Added exhaustive match |

**Total**: 54 errors fixed

---

## 📊 Testing All Modes

```powershell
# Classical (no features needed)
cargo run -p hello_companion --release

# BehaviorTree
cargo run -p hello_companion --release --features llm,ollama -- --bt

# Utility AI
cargo run -p hello_companion --release --features llm,ollama -- --utility

# Real Phi-3
cargo run -p hello_companion --release --features llm,ollama -- --llm

# Hybrid (LLM + fallback)
cargo run -p hello_companion --release --features llm,ollama -- --hybrid

# Ensemble (voting)
cargo run -p hello_companion --release --features llm,ollama -- --ensemble

# All modes with metrics
cargo run -p hello_companion --release --features llm,ollama,metrics -- --demo-all --metrics --export-metrics
```

---

## 🐛 Troubleshooting

**If compilation fails**:
```powershell
cargo clean -p hello_companion
cargo check -p hello_companion --features llm,ollama
```

**If Ollama not found**:
```powershell
ollama serve  # Start Ollama
ollama pull phi3  # Download model
```

---

## 📁 Files

- **HELLO_COMPANION_FIXED.txt** - Complete corrected code
- **HELLO_COMPANION_FIX_SUMMARY.md** - Detailed analysis
- **examples/hello_companion/Cargo.toml** - Already updated ✅

---

**Next**: Copy code from HELLO_COMPANION_FIXED.txt → main.rs and test!
