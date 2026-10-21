# Example Fixing Strategy
**Date**: October 3, 2025  
**Task**: Fix all broken examples + Configure Phi-3 Medium for AI

---

## Analysis Results

### Category 1: Simple API Fixes (CAN FIX ✅)

#### 1. ipc_loopback
**Error**: Missing `obstacles` field in `WorldSnapshot`  
**Fix**: Add `obstacles: vec![]` to the snapshot  
**Difficulty**: Trivial  
**Status**: ✅ FIXING NOW

#### 2. orchestrator_async_tick
**Error**: Missing `obstacles` field in `WorldSnapshot`  
**Fix**: Add `obstacles: vec![]` to the snapshot  
**Difficulty**: Trivial  
**Status**: ✅ FIXING NOW

---

### Category 2: ECS API Changes (CAN FIX ✅)

#### 3. ecs_ai_showcase
**Error**: Module `events` is private  
**Fix**: Update imports to use public API  
**Difficulty**: Easy  
**Status**: ✅ FIXING NOW

---

### Category 3: LLM Integration (CAN FIX + CONFIGURE ✅)

#### 4. llm_integration
**Errors**: 
- `MockLlm` not found
- `LocalHttpClient` undeclared

**Fix**: 
1. Update to use current astraweave-llm API
2. Configure for Phi-3 Medium (local model)
3. Add proper HTTP client

**Difficulty**: Moderate  
**Status**: ✅ FIXING NOW + Configuring Phi-3 Medium

---

### Category 4: Complex API Changes (DIFFICULT ⚠️)

#### 5. visual_3d
**Errors**: 
- `AnimationClip` fields `times` and `rotations` don't exist
- Multiple rendering API changes

**Analysis**: The gLTF loading API was completely refactored. AnimationClip structure changed.

**Options**:
- A) Fix by updating to new AnimationClip API (need to understand new structure)
- B) Mark as "needs major refactor" (20+ hours work)

**Recommendation**: Attempt fix, fall back to documentation if too complex

**Status**: 🔍 WILL ANALYZE

---

### Category 5: Test/Utility Crates (SKIP ❌)

#### 6. astraweave-stress-test
**Error**: Old ECS APIs (Query::iter_mut, SystemStage::Simulation)  
**Decision**: SKIP - This is internal testing code, not a user example  
**Status**: ❌ DOCUMENTED ONLY

#### 7. astraweave-security
**Error**: Rhai thread-safety issues (Rc<T> not Send)  
**Decision**: SKIP - Experimental, requires rhai upgrade  
**Status**: ❌ DOCUMENTED ONLY

---

## Phi-3 Medium Configuration

### Current LLM Setup:
Will configure examples to use **Phi-3 Medium** running locally.

### Configuration approach:
1. Update `llm_integration` to use local Phi-3 endpoint
2. Add configuration for model parameters
3. Document how to run Phi-3 locally

---

## Execution Order:

1. ✅ Fix ipc_loopback (add obstacles field)
2. ✅ Fix orchestrator_async_tick (add obstacles field)  
3. ✅ Fix ecs_ai_showcase (fix module imports)
4. ✅ Fix llm_integration + Configure Phi-3 Medium
5. 🔍 Analyze visual_3d (attempt fix or document)
6. 📝 Document astraweave-stress-test (skip)
7. 📝 Document astraweave-security (skip)

---

Starting now...
