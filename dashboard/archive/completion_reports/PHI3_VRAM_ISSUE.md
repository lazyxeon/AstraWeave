# Phi-3 Performance Issue - VRAM Limitation

## Problem
Your GTX 1660 Ti has **6GB VRAM**, but:
- `phi3:medium` needs ~8GB → **CPU fallback** (55-140s latency!)
- `phi3:3.8b` (mini) needs ~2.3GB → Should work on GPU but still slow (6s)

## Root Cause
Ollama is splitting the model between CPU/GPU (63% CPU / 37% GPU).

## Solutions

### Option 1: Use phi3:mini (Recommended)
```bash
# Create optimized mini model
ollama create phi3:game -f Modelfile.phi3-game
```

Expected latency: **0.5-1.5s** (acceptable for game AI)

### Option 2: Upgrade GPU (Future)
- RTX 3060 (12GB): Can run phi3:medium fully on GPU
- RTX 4060 Ti (16GB): Ideal for AI development

### Option 3: Reduce Model Layers on GPU
Force more aggressive quantization:
```bash
OLLAMA_NUM_GPU_LAYERS=20 ollama serve
```

## Current Performance
| Model | Size | VRAM | CPU/GPU Split | Latency |
|-------|------|------|---------------|---------|
| phi3:medium | 14GB | 6GB | 63%/37% | 55-140s ❌ |
| phi3:3.8b | 2.3GB | 6GB | Unknown | 6s ⚠️ |
| phi3:game (opt) | 2.3GB | 6GB | 100% GPU | 0.5-1.5s ✅ (target) |

## Immediate Action
Using `phi3:3.8b` (mini) as default for demos - works within VRAM limits.
