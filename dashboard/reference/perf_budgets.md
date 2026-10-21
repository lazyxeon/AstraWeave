# Performance Budgets (MVP → Phase 3)

Purpose: Concrete ms budgets per subsystem and phase; informs benches and CI gates.

## Phase 0 Budgets (1080p, mid-tier GPU)
- CPU (per tick at 60 Hz):
  - Physics + Nav: ≤ 1.5 ms
  - AI (fast-think only): ≤ 1.0 ms
  - Audio + Misc: ≤ 0.5 ms
- GPU (frame):
  - Geometry + PBR: ≤ 5.0 ms
  - Shadows (CSM 2 cascades): ≤ 1.5 ms
  - Post (ACES, simple bloom): ≤ 0.5 ms
- Asset Import SLA: ≤ 1.0 s for a 100k tri glTF (cold); ≤ 200 ms warm cache
- Shader Compile (per program): ≤ 150 ms cold; ≤ 20 ms warm
- Hot Reload budget: ≤ 500 ms material param/WGSL change → on-screen update

## Phase 1 Budgets
- Add: Dialogue/TTS: ≤ 10 ms scheduling on audio thread (synthesis off-thread)
- AI Deep-Think budget: ≤ 150 ms (fallback micro-policy if exceeded)

## CI Gates
- Criterion.rs: fail on >200% regression of baseline medians
- Renderer: golden image ΔE ≤ 2.0 after material/shader changes (tolerance windowed)
- `make ci`: enforce budgets via bench thresholds in `docs/BENCHMARKING_GUIDE.md`
