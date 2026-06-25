# AstraWeave Fluids: Research-Grade Enhancement Plan

> **⚠ STALENESS BANNER (added W-series W.2 Phase 2, 2026-06-21).** This roadmap
> targets **research-grade multi-solver SPH** (DFSPH/PCISPH/IISPH, multi-phase,
> non-Newtonian, turbulence, 500k–1M particles) — an inventory the W-series
> **deleted** in W.1 (commit `1a57fdd41`; recovery tag `w0-pre-deprecation`). The
> F→W deprecation re-scoped water from a general fluid *simulation* to a layered
> *rendering* system; the only retained particle path is the F.4 Option-A accent
> substrate. The roadmap below is **not pursued** — the current authority is
> `docs/campaigns/water-successor/W2_0_RECON.md` and `W2_DECISIONS.md`. **Read
> this as historical; do not cite its targets as current.**

**Version**: 2.0  
**Date**: January 2026  
**Status**: 📋 Comprehensive Audit & Roadmap (Revised with Expert Review)  
**Target**: Water and fluids systems capable of research-grade simulations  
**Reviewed By**: External AI review (Grok 4) with state-of-the-art 2024-2026 research

---

## Executive Summary

This document presents a comprehensive audit of the AstraWeave fluids system and a detailed enhancement roadmap to achieve **research-grade fluid simulation** capable of accurately simulating:

- **Water** (low viscosity, 0.001 Pa·s)
- **Oils** (medium-high viscosity, 0.01-1.0 Pa·s)
- **Honey/Syrups** (high viscosity, 2-10 Pa·s)
- **Multi-phase interactions** (oil-water separation, emulsions)
- **Non-Newtonian fluids** (shear-thinning/thickening)
- **Turbulent flows** (vorticity-rich splashes, swirling)

### Industry Context (2026)

**Gold Standard References**:
- **SPlisHSPlasH** (RWTH Aachen): Research SPH with DFSPH, IISPH, PCISPH, implicit viscosity, multi-phase, partial CUDA acceleration
- **Taichi**: GPU-optimized sparse structures, millions of particles in real-time for SPH/MPM
- **VFX Tools** (Houdini FLIP/APIC, Bifrost): Hybrid Eulerian-Lagrangian for superior detail
- **Game Engines** (UE5 Water, Niagara): FFT/shallow-water + simplified particles (not full SPH)

**Reality Check**: Pure SPH is rarely "world-class" in games without hybrids or approximations. Optimized GPU SPH reaches ~500k-1M particles at 30-60fps with trade-offs.

### Current State Assessment

| Aspect | Current Grade | Target Grade | Gap | Notes |
|--------|---------------|--------------|-----|-------|
| **Solver Accuracy** | B (PBD) | A+ (DFSPH/PCISPH) | Medium | Add PCISPH option |
| **Viscosity Models** | C+ (XSPH only) | A+ (Morris/Matrix-free) | High | Matrix-free, not full CG |
| **Incompressibility** | B- (soft constraint) | A+ (divergence-free) | High | With δ-SPH stabilization |
| **Multi-Phase** | B (basic phase field) | A (δ⁺-SPH interfaces) | Medium | Sharper interfaces needed |
| **Surface Tension** | B+ (Akinci cohesion) | A+ (CSF + curvature smooth) | Low | Add noise reduction |
| **Boundary Handling** | B (particle + SDF) | A (Hybrid SDF+Akinci) | Medium | Optimize resampling |
| **GPU Performance** | A (128k particles) | A (300-500k realistic) | Low | 1M with LOD/hybrid only |
| **Turbulence/Vorticity** | D (basic confinement) | A (micropolar/enrichment) | High | Critical for realism |
| **Research Features** | D (none) | A+ (validation suite) | High | Add vortex ring tests |

**Overall Current Grade: B (Good for games, insufficient for research)**

### Realistic Performance Targets (Revised)

| Quality Tier | Particle Count | Solver | FPS Target | Use Case |
|--------------|---------------|--------|------------|----------|
| **Low** | 50-100k | PBD | 60+ fps | Mobile, background |
| **Medium** | 100-200k | PCISPH | 60 fps | Standard gameplay |
| **High** | 200-500k | DFSPH | 30-60 fps | Hero fluids, AAA |
| **Research** | 100-300k | DFSPH+Implicit | 15-30 fps | Validation, offline |
| **Hybrid** | 500k-1M | PBD+heightfield | 30-60 fps | Large-scale water |

> ⚠️ **Note**: Previous 1M+ target with full DFSPH was overly optimistic. Realistic high-quality target is 300-500k particles with advanced features, or 1M with LOD/hybrid approaches.

---

## Part 1: Current Implementation Audit

### 1.1 Solver Architecture

**Technology**: Position-Based Dynamics (PBD) / Position-Based Fluids (PBF)

**Strengths**:
- ✅ GPU-accelerated via WGPU compute shaders
- ✅ Stable with large timesteps (great for games)
- ✅ Good visual results for real-time applications
- ✅ Spatial hash grid for O(n) neighbor search (128³ grid)
- ✅ Multi-phase support (water=0, oil=1, custom=2)

**Weaknesses**:
- ❌ Not truly incompressible (density error ~1-5%)
- ❌ PBD convergence depends on iteration count (not physical)
- ❌ No velocity divergence constraint
- ❌ Viscosity via XSPH (too simple for research)
- ❌ No pressure field for accurate viscosity coupling

**Current Shader Pipeline** (from `fluid.wgsl`):
```
predict → clear_grid → build_grid → compute_lambda → compute_delta_pos → integrate → mix_dye
```

### 1.2 Viscosity Handling

**Current Method**: XSPH viscosity (lines 402-420 in `fluid.wgsl`)
```wgsl
// Current XSPH implementation
xsph_vel += 0.01 * (neighbor_vel - vel) * kernel_w(r, h);
```

**Issues**:
- XSPH is artificial viscosity (not physically accurate)
- Single hardcoded coefficient (0.01)
- No temperature-dependent viscosity
- No shear-rate dependency for non-Newtonian fluids
- Cannot simulate oils, honey, or thick fluids accurately

**Research Target**: Morris viscosity model + implicit integration
```
∂v/∂t = μ/ρ ∇²v  (Laplacian of velocity field)
```

### 1.3 Multi-Phase Support

**Current Method**: Phase field (integer per particle)
```rust
pub phase: u32,  // 0=water, 1=oil, 2=custom
```

**Surface Tension** (lines 247-255 in `fluid.wgsl`):
```wgsl
// Akinci 2013 cohesion (simplified)
let cohesion = -params.surface_tension * cohesion_weight * normalize(diff);
```

**Issues**:
- No phase-specific density/viscosity
- Missing interface tension between phases
- No curvature-based surface tension
- Immiscible fluids don't separate properly

### 1.4 Incompressibility

**Current Method**: Soft density constraint via PBD
```wgsl
let constraint = (density / params.target_density) - 1.0;
let epsilon = 100.0;  // Large softening factor
let lambda = -constraint / (sum_grad_c2 + epsilon);
```

**Issues**:
- Epsilon (100.0) is too large for research accuracy
- No divergence-free velocity constraint
- Density oscillates under compression

**Research Target**: DFSPH or IISPH for <0.1% density error

### 1.5 Boundary Handling

**Current Methods**:
1. SDF collision (good)
2. Simple position clamping (bad)
3. Dynamic object collision via inverse transform

**Issues**:
- Missing Akinci boundary particle method
- Friction model is basic
- No slip/no-slip boundary conditions

### 1.6 Production Features (Strengths)

**Excellent existing infrastructure**:
- ✅ Screen-space fluid rendering (SSFR)
- ✅ Caustics, god rays, underwater effects
- ✅ Foam and whitewater system
- ✅ LOD and quality presets
- ✅ Profiling and optimization metrics
- ✅ GPU vendor-aware workgroup tuning
- ✅ Adaptive iteration controller
- ✅ Serialization for save/load

### 1.7 Critical Missing Features (Identified in Review)

**Not in current implementation**:
- ❌ **Tensile Instability Correction**: No particle shifting (δ-SPH), leading to clumping/voids
- ❌ **Vorticity Enrichment**: Only basic confinement, produces overly laminar flows
- ❌ **Heat Diffusion/Advection**: Temperature only affects viscosity, no thermal transport
- ❌ **Turbulence Model**: No micropolar or turbulence particles for realistic splashes
- ❌ **Warm-Starting**: Solvers start fresh each frame (inefficient)
- ❌ **Air Phase**: No implicit air for splashes/bubbles
- ❌ **FLIP/APIC Hybrid**: Pure particle, no grid transfer option

---

## Part 2: Research-Grade Enhancement Roadmap

### Phase 1: Solver Upgrade (4-6 weeks)

**Goal**: Upgrade from PBD to DFSPH/PCISPH with stability enhancements

#### 1.1 DFSPH Implementation (Divergence-Free SPH)

**Reference**: Bender & Koschier 2015, 2017

**New Shader Pipeline**:
```
predict → clear_grid → build_grid →
  [compute_density_alpha] →           # NEW: α factor for density solve
  [solve_density_error] →              # NEW: Jacobi iterations for Δρ=0
  [compute_divergence_factor] →        # NEW: κ factor for velocity divergence
  [solve_velocity_divergence] →        # NEW: Jacobi iterations for ∇·v=0
  [apply_viscosity] →                  # MOVED: Before integration for stability
  integrate → particle_shift → mix_dye
```

> ⚠️ **Critical Fix**: Viscosity must be applied BEFORE or WITHIN pressure solves, not after integration. This is the standard in SPlisHSPlasH and prevents high-viscosity instability.

**Key Equations**:
```
// Density error correction
Δv_i = (1/Δt²) Σⱼ (κ_i + κⱼ) ∇W_ij

// Divergence-free correction  
Δv_i = (1/Δt) Σⱼ (α_i + αⱼ) ∇W_ij
```

**New Buffers Required**:
- `alpha_factors: Buffer<f32>` - Density error factor per particle
- `kappa_factors: Buffer<f32>` - Divergence factor per particle
- `velocity_divergence: Buffer<f32>` - ∇·v per particle
- `previous_pressure: Buffer<f32>` - For warm-starting (NEW)

**Expected Improvement**: Density error 5% → <0.1%

#### 1.2 PCISPH Alternative (Predictive-Corrective)

**Reference**: Solenthaler & Pajarola 2009

**Why Add This**: Often faster convergence than DFSPH in real-time scenarios, simpler to implement.

```rust
pub enum IncompressibilitySolver {
    PBD,      // Current (fast, visual)
    PCISPH,   // Predictive-Corrective (balanced)
    DFSPH,    // Divergence-Free (accurate)
    IISPH,    // Implicit (most stable, slowest)
}
```

**PCISPH Pipeline**:
```
predict → clear_grid → build_grid →
  loop until converged:
    [compute_density_error] →
    [compute_pressure_correction] →
  [apply_pressure_forces] →
  integrate → particle_shift
```

#### 1.3 Particle Shifting (δ-SPH / δ⁺-SPH) — CRITICAL ADDITION

**Reference**: Marrone et al. 2011, Sun et al. 2017 (δ⁺-SPH)

**Problem Solved**: Standard SPH suffers from **tensile instability** (particle clumping) and **void formation** under stretching. This is NOT addressed in basic DFSPH.

**δ-SPH Shifting Formula**:
```wgsl
// Particle shifting to maintain uniform distribution
let shift_i = -C_δ * h² * Σⱼ (1 + 0.2 * (W_ij/W_0)⁴) * ∇W_ij

// Apply with free-surface correction
if (!is_surface_particle) {
    particles[id].position += shift_i * dt;
}
```

**δ⁺-SPH Enhancement** (for multi-phase):
```wgsl
// Interface-aware shifting (prevents mixing at phase boundaries)
let phase_gradient = compute_phase_gradient(id);
let shift_corrected = shift_i - dot(shift_i, phase_gradient) * phase_gradient;
```

**New Shader**: `particle_shifting.wgsl`

#### 1.4 Warm-Starting & Adaptive Convergence

**Problem**: Fixed iteration counts (e.g., 100) kill real-time performance.

**Solution**: Error-based early exit + warm-starting from previous frame.

```rust
pub struct AdaptiveSolverConfig {
    pub min_iterations: u32,           // 1-3 for games
    pub max_iterations: u32,           // 50-100 for research
    pub density_error_threshold: f32,  // 0.001 (0.1%)
    pub enable_warm_start: bool,       // Reuse previous pressure
}
```

**Warm-Starting Shader**:
```wgsl
// Initialize pressure from previous frame (70-90% fewer iterations needed)
let warm_pressure = previous_pressure[id] * warm_start_factor;
particles[id].pressure = warm_pressure;
```

### Phase 2: Advanced Viscosity (3-4 weeks)

**Goal**: Accurate viscosity for oils, honey, and temperature-dependent fluids

#### 2.1 Morris Viscosity Model

**Reference**: Morris et al. 1997

**Equation**:
```
(∂v/∂t)_viscosity = Σⱼ mⱼ (μ_i + μⱼ)/(ρ_i ρⱼ) (v_i - v_j) / (|r_ij|² + 0.01h²) ∇W_ij · r_ij
```

**New Shader** (`viscosity_morris.wgsl`):
```wgsl
@compute @workgroup_size(64)
fn compute_viscosity(@builtin(global_invocation_id) gid: vec3<u32>) {
    let id = gid.x;
    if (id >= params.particle_count) { return; }
    
    let pos = particles[id].position.xyz;
    let vel = particles[id].velocity.xyz;
    let rho_i = particles[id].density;
    let mu_i = get_viscosity(particles[id].phase, particles[id].temperature);
    
    var visc_force = vec3<f32>(0.0);
    
    // Neighbor iteration...
    for each neighbor j {
        let diff = pos - neighbor_pos;
        let r = length(diff);
        if (r < h) {
            let mu_j = get_viscosity(particles[j].phase, particles[j].temperature);
            let rho_j = particles[j].density;
            
            // Morris formula
            let denom = r * r + 0.01 * h * h;
            let grad_dot_r = dot(kernel_grad_w(r, diff, h), diff);
            let factor = (mu_i + mu_j) / (rho_i * rho_j * denom) * grad_dot_r;
            
            visc_force += factor * (vel - particles[j].velocity.xyz);
        }
    }
    
    particles[id].velocity += vec4<f32>(visc_force * params.dt, 0.0);
}
```

#### 2.2 Matrix-Free Implicit Viscosity Solver

> ⚠️ **Critical Fix from Expert Review**: Full CG solvers are **infeasible on GPUs** for >100k particles due to memory bandwidth and global sync overhead. Use **matrix-free Jacobi** instead.

For high viscosity (μ > 1.0 Pa·s), explicit integration is unstable.

**Reference**: Weiler et al. 2018, Peer et al. 2015

**Matrix-Free Method** (GPU-efficient):
```
v^(n+1) = Jacobi_iterate(v^n, viscosity_operator)
```

**No explicit matrix construction required** - viscosity operator applied directly per particle.

**Implementation**:
```wgsl
@compute @workgroup_size(128)
fn implicit_viscosity_jacobi(@builtin(global_invocation_id) gid: vec3<u32>) {
    let id = gid.x;
    if (id >= params.particle_count) { return; }
    
    let pos_i = particles[id].position.xyz;
    let vel_old = velocity_in[id];
    let mu_i = get_viscosity(particles[id].phase, particles[id].temperature);
    
    var weighted_sum = vec3<f32>(0.0);
    var weight_total = 0.0;
    
    // Neighbor iteration
    for each neighbor j {
        let mu_j = get_viscosity(particles[j].phase, particles[j].temperature);
        let mu_avg = 2.0 * mu_i * mu_j / (mu_i + mu_j + 1e-8);
        
        let laplacian_W = laplacian_kernel(r, h);
        let weight = params.dt * mu_avg * laplacian_W * particles[j].mass / particles[j].density;
        
        weighted_sum += weight * velocity_in[j];
        weight_total += weight;
    }
    
    // Jacobi update (no matrix needed!)
    velocity_out[id] = (vel_old + weighted_sum) / (1.0 + weight_total);
}
```

**Iteration Config**:
```rust
pub struct ImplicitViscosityConfig {
    pub max_iterations: u32,    // 5-10 for games, 20-50 for research
    pub tolerance: f32,         // 1e-4 typical
    pub omega: f32,             // SOR relaxation: 0.5-0.8
}
```

**Why This Works Better**:
- O(n) memory (no sparse matrix storage)
- Embarrassingly parallel
- No global reductions per iteration
- Converges in 3-10 iterations for most scenarios

#### 2.3 Vorticity-Based Shear Rate Estimation

**Problem Identified in Review**: Computing shear rate from velocity gradients is **noisy** with standard SPH particle distributions.

**Solution**: Use vorticity magnitude as smoothed proxy.

```wgsl
fn compute_robust_shear_rate(id: u32) -> f32 {
    // Vorticity-based (smoother)
    let omega = compute_vorticity(id);
    let vort_mag = length(omega);
    
    // Strain tensor-based (more accurate but noisier)
    let strain = compute_strain_tensor(id);
    let strain_mag = sqrt(2.0 * second_invariant(strain));
    
    // Blend with bias toward vorticity for stability
    return 0.7 * vort_mag + 0.3 * strain_mag;
}
```

#### 2.4 Non-Newtonian Fluids (Enhanced)

**Shear-Thinning** (ketchup, paint):
```
μ_eff = μ_0 * (1 + (λ γ̇)²)^((n-1)/2)
```

**Shear-Thickening** (cornstarch suspension):
```
μ_eff = μ_0 * (1 + (λ γ̇)²)^((n-1)/2)  where n > 1
```

**New Per-Particle Fields**:
```rust
pub struct NonNewtonianParams {
    pub viscosity_0: f32,      // Reference viscosity
    pub power_index: f32,      // n (< 1 = thinning, > 1 = thickening)
    pub consistency: f32,      // λ
}
```

#### 2.4 Temperature-Dependent Viscosity

**Arrhenius Model**:
```
μ(T) = A * exp(E_a / (R * T))
```

**Simpler VTF Model**:
```
μ(T) = μ_ref * exp(B * (1/T - 1/T_ref))
```

**New Config**:
```rust
pub struct PhaseViscosityConfig {
    pub reference_viscosity: f32,   // μ at T_ref
    pub reference_temp: f32,        // T_ref in Kelvin
    pub activation_energy: f32,     // Temperature sensitivity
}
```

### Phase 3: Multi-Phase Enhancement (3-4 weeks)

**Goal**: Accurate oil-water separation, emulsions, air bubbles, and interface dynamics

> 🆕 **Expert Enhancement**: Add δ⁺-SPH interface sharpening and implicit air phase for proper splash/bubble behavior.

#### 3.1 Per-Phase Properties

**New Struct**:
```rust
pub struct FluidPhase {
    pub id: u32,
    pub name: String,
    pub rest_density: f32,          // kg/m³ (water=1000, oil=800, air=1.2)
    pub viscosity: f32,              // Pa·s (water=0.001, oil=0.1)
    pub surface_tension: f32,        // N/m
    pub color: [f32; 4],
    pub miscible_with: Vec<u32>,     // Which phases can mix
    pub is_gas: bool,                // NEW: For air phase handling
}
```

**Presets (Expanded)**:
```rust
pub enum FluidPreset {
    Water,          // ρ=1000, μ=0.001, σ=0.072
    SeaWater,       // ρ=1025, μ=0.00108
    VegetableOil,   // ρ=920, μ=0.04-0.06
    MotorOil,       // ρ=880, μ=0.1-0.3
    Honey,          // ρ=1400, μ=2-10
    Glycerin,       // ρ=1260, μ=1.5
    Lava,           // ρ=2500, μ=100-10000, T-dependent
    Air,            // ρ=1.2, μ=1.8e-5, implicit phase (NEW)
    Foam,           // Spawned at splash sites (NEW)
}
```

#### 3.2 δ⁺-SPH Interface Sharpening — CRITICAL ADDITION

**Reference**: Sun et al. 2017 "A consistent approach to particle shifting"

**Problem**: Standard multi-phase SPH leads to interface diffusion and mixing artifacts.

**Solution**: δ⁺-SPH applies particle shifting only PARALLEL to interfaces, not across them.

```wgsl
// Phase-aware particle shifting (δ⁺-SPH)
fn compute_interface_aware_shift(id: u32) -> vec3<f32> {
    let base_shift = compute_base_shift(id);  // Standard δ-SPH
    
    // Compute phase gradient (interface normal)
    let phase_gradient = compute_phase_gradient(id);
    let grad_magnitude = length(phase_gradient);
    
    if (grad_magnitude > INTERFACE_THRESHOLD) {
        // Near interface: project shift to be tangent
        let normal = phase_gradient / grad_magnitude;
        let tangent_shift = base_shift - dot(base_shift, normal) * normal;
        return tangent_shift;
    } else {
        // Away from interface: full shift allowed
        return base_shift;
    }
}
```

**Interface Detection**:
```wgsl
fn compute_phase_gradient(id: u32) -> vec3<f32> {
    let my_phase = particles[id].phase;
    var gradient = vec3<f32>(0.0);
    
    for each neighbor j {
        let phase_diff = f32(particles[j].phase != my_phase);
        gradient += phase_diff * kernel_gradient(pos_i - pos_j, h);
    }
    
    return gradient;
}
```

#### 3.3 Implicit Air Phase — CRITICAL ADDITION

**Problem**: Pure water simulations can't capture splashes, bubbles, or foam realistically.

**Solution**: Treat air as an implicit phase with SPH particles spawned dynamically.

```rust
pub struct ImplicitAirConfig {
    pub enable_air_phase: bool,
    pub spawn_at_free_surface: bool,    // Create air particles at surface
    pub spawn_threshold: f32,           // Velocity threshold for splash
    pub max_air_particles: u32,         // Budget for air particles
    pub air_buoyancy: f32,              // Upward force on air bubbles
    pub bubble_lifetime: f32,           // Seconds before removal
}
```

**Air Particle Spawning**:
```wgsl
// Spawn air at high-velocity surface impacts
if (is_surface_particle && velocity_magnitude > params.splash_threshold) {
    // Spawn 2-4 air particles in splash direction
    for (var i = 0u; i < 3u; i++) {
        let air_particle = create_air_particle(
            pos + random_offset(),
            vel * 0.3 + random_spread()
        );
        emit_particle(air_particle);
    }
}
```

**Bubble Physics**:
```wgsl
// Air particles experience buoyancy
if (particles[id].is_gas) {
    let buoyancy = (liquid_density - air_density) * volume * gravity;
    particles[id].velocity.y += buoyancy * dt;
    
    // Air-water drag
    particles[id].velocity *= 0.98;  // Simple drag
}
```

#### 3.4 Interface Tension (Akinci Method)

**Reference**: Akinci et al. 2013 "Versatile Surface Tension and Adhesion"

**Cohesion Force** (same phase):
```
F_cohesion = -γ m² C(r) (x_i - x_j) / |x_i - x_j|
```

**Adhesion Force** (different phase):
```
F_adhesion = -β m² A(r) (x_i - x_j) / |x_i - x_j|
```

**Interface Tension Matrix**:
```rust
pub struct InterfaceTension {
    pub tension_matrix: [[f32; MAX_PHASES]; MAX_PHASES],  // γ_ij values
    pub adhesion_matrix: [[f32; MAX_PHASES]; MAX_PHASES], // β_ij values
}
```

#### 3.3 Curvature-Based Surface Tension (CSF)

**Reference**: Brackbill et al. 1992 (Continuum Surface Force)

**Color Field Gradient**:
```
n_i = Σⱼ (c_j / ρ_j) ∇W_ij
```

**Curvature**:
```
κ_i = -∇·n̂_i = -Σⱼ (n̂_j - n̂_i) / ρ_j ∇W_ij · r_ij / (|r_ij|² + ε)
```

**Surface Tension Force**:
```
F_st = -σ κ n̂
```

### Phase 4: Boundary Handling Upgrade (2-3 weeks)

**Goal**: Accurate solid-fluid interaction with slip/no-slip control and efficient sampling

> 🆕 **Expert Enhancement**: Add SDF-based density contribution and hybrid SDF + Akinci boundaries to reduce resampling cost.

#### 4.1 Akinci Boundary Particles

**Reference**: Akinci et al. 2012

**Method**: Sample boundary surfaces with particles
```rust
pub struct BoundaryParticle {
    pub position: [f32; 3],
    pub volume: f32,           // Ψ (boundary contribution)
    pub normal: [f32; 3],
    pub friction: f32,
}
```

**Density Contribution**:
```
ρ_i += Σ_b ρ_0 Ψ_b W(x_i - x_b, h)
```

#### 4.2 Hybrid SDF + Akinci Boundaries — CRITICAL ADDITION

**Problem Identified**: Full Akinci resampling for complex geometry is expensive (O(n_boundary) per fluid particle).

**Solution**: Use SDF for density contribution, Akinci particles only for friction/adhesion.

```rust
pub enum BoundaryMethod {
    AkinciOnly,       // Traditional particle sampling
    SDFOnly,          // Fast but less accurate friction
    Hybrid {          // Recommended
        sdf_for_density: bool,
        particles_for_friction: bool,
    },
}
```

**SDF-Based Density Contribution**:
```wgsl
// Much faster than iterating boundary particles
fn boundary_density_from_sdf(pos: vec3<f32>) -> f32 {
    let dist = sample_sdf(pos);
    
    if (dist > h) { return 0.0; }  // Far from boundary
    
    // Approximate density contribution based on distance
    let overlap = h - dist;
    let volume_fraction = overlap / h;
    
    return rest_density * volume_fraction * kernel_at_distance(dist);
}
```

**Sparse Boundary Particles**:
```rust
pub struct SparseAkinciConfig {
    pub particle_spacing: f32,      // Larger spacing than fluid particles
    pub only_at_corners: bool,      // Dense sampling only at sharp features
    pub adaptive_sampling: bool,    // Refine based on fluid proximity
}
```

#### 4.3 Slip/No-Slip Boundaries

**No-Slip** (default): Zero relative velocity at boundary
```
v_boundary = v_solid
```

**Free-Slip**: Only normal velocity zeroed
```
v_boundary = v - (v · n) n
```

**Partial-Slip**:
```
v_boundary = α v + (1-α) (v - (v·n) n)
```

### Phase 5: Turbulence & Vorticity Enrichment (2-3 weeks) — NEW PHASE

**Goal**: Realistic turbulent splashes, vortex dynamics, and small-scale detail

> 🆕 **Expert Enhancement**: This was identified as completely missing from the original plan. Critical for visual realism.

#### 5.1 Vorticity Confinement

**Reference**: Fedkiw et al. 2001 (originally for smoke), Müller et al. 2007 (for SPH)

**Problem**: Numerical dissipation destroys small-scale vortices in SPH. Water looks "too calm."

**Solution**: Re-inject lost vorticity based on computed curl.

**Vorticity Computation**:
```wgsl
fn compute_vorticity(id: u32) -> vec3<f32> {
    var omega = vec3<f32>(0.0);
    let pos_i = particles[id].position.xyz;
    let vel_i = particles[id].velocity.xyz;
    
    for each neighbor j {
        let r = pos_i - pos_j;
        let v_diff = particles[j].velocity.xyz - vel_i;
        let grad_W = kernel_gradient(r, h);
        
        // ω = ∇ × v via SPH
        omega += particles[j].mass / particles[j].density * cross(v_diff, grad_W);
    }
    
    return omega;
}
```

**Confinement Force**:
```wgsl
fn compute_vorticity_confinement(id: u32) -> vec3<f32> {
    let omega = vorticity[id];
    let omega_mag = length(omega);
    
    if (omega_mag < 1e-6) { return vec3<f32>(0.0); }
    
    // Gradient of vorticity magnitude (normalized)
    var grad_omega_mag = vec3<f32>(0.0);
    for each neighbor j {
        let omega_j_mag = length(vorticity[j]);
        grad_omega_mag += (omega_j_mag - omega_mag) * kernel_gradient(...);
    }
    
    let N = normalize(grad_omega_mag + vec3<f32>(1e-8));  // Vorticity direction
    
    // Confinement force: F = ε (N × ω)
    return params.vorticity_epsilon * cross(N, omega);
}
```

**Configuration**:
```rust
pub struct VorticityConfinementConfig {
    pub epsilon: f32,           // Strength (0.01-0.1 typical)
    pub scale_with_velocity: bool,
    pub apply_to_surface_only: bool,
}
```

#### 5.2 Micropolar SPH (Particle Spin)

**Reference**: Bender et al. 2017 "Micropolar SPH"

**Concept**: Each particle has angular velocity (spin), enabling fine-scale rotational effects.

**New Particle Fields**:
```rust
pub struct MicropolarParticle {
    // ... existing fields ...
    pub angular_velocity: [f32; 3],  // ω (spin)
    pub moment_of_inertia: f32,      // I (depends on particle radius)
}
```

**Angular Momentum Transfer**:
```wgsl
// Transfer between linear and angular momentum
let torque = compute_particle_torque(id);
particles[id].angular_velocity += torque * dt / moment_of_inertia;

// Angular velocity affects neighbors
let spin_contribution = cross(angular_velocity, (pos_j - pos_i));
velocity_correction += micropolar_coupling * spin_contribution;
```

**Benefits**:
- More realistic splash breakup
- Better small-scale turbulence
- Improved mixing dynamics

#### 5.3 Turbulence Particles

**Reference**: Thuerey et al. 2010 "Turbulent Particles"

**Concept**: Add passive tracer particles that amplify perceived turbulence without changing dynamics.

```rust
pub struct TurbulenceParticles {
    pub positions: Vec<[f32; 3]>,
    pub velocities: Vec<[f32; 3]>,
    pub lifetime: Vec<f32>,
    pub spawn_near_surface: bool,
    pub spawn_near_vortices: bool,
}
```

**Shader**:
```wgsl
// Spawn turbulence particles near high-vorticity regions
if (vorticity_magnitude > params.turb_spawn_threshold) {
    spawn_turbulence_particle(pos, vel + noise_offset);
}
```

**Visual-only**: These don't affect simulation, purely for rendering detail.

### Phase 6: Validation & Research Features (2-3 weeks)

**Goal**: Enable research-grade validation and comparison

#### 5.1 Validation Suite

**Standard Tests**:
1. **Dam Break**: Compare with experimental data (Martin & Moyce 1952)
2. **Hydrostatic Pressure**: Verify pressure = ρgh
3. **Couette Flow**: Viscosity validation (linear velocity profile)
4. **Poiseuille Flow**: Pipe flow validation (parabolic profile)
5. **Rayleigh-Taylor**: Density-driven instability
6. **Drop Splash**: Surface tension validation
7. **Oil-Water Separation**: Multi-phase validation

**Metrics Export**:
```rust
pub struct ValidationMetrics {
    pub density_error_max: f32,
    pub density_error_avg: f32,
    pub divergence_error_max: f32,
    pub divergence_error_avg: f32,
    pub energy_conservation: f32,
    pub momentum_conservation: [f32; 3],
    pub mass_conservation: f32,
}
```

#### 5.2 Comparison Framework

**Reference Data**:
- Load experimental CSV (position, velocity over time)
- Compare against simulation
- Report RMSE, peak error

**Academic Output**:
- Export to VTK for ParaView visualization
- PLY/OBJ mesh export for surface
- JSON simulation state snapshots

#### 5.3 Parameter Study Support

```rust
pub struct ParameterStudy {
    pub parameter_name: String,
    pub values: Vec<f32>,
    pub baseline_config: FluidConfig,
    pub metrics_to_track: Vec<String>,
}

impl ParameterStudy {
    pub fn run(&self) -> Vec<StudyResult>;
    pub fn export_csv(&self, path: &Path);
}
```

---

## Part 3: Implementation Priority Matrix (Revised v2.0)

> 🔄 **Updated based on expert review**: Added particle shifting, vorticity, matrix-free viscosity, and revised timelines.

### Critical Path (Must Have for Research Grade)

| Priority | Feature | Effort | Impact | Dependencies | Notes |
|----------|---------|--------|--------|--------------|-------|
| **P0** | DFSPH/PCISPH Solver | 4-5 weeks | ⭐⭐⭐⭐⭐ | None | Start with PCISPH (simpler), then DFSPH |
| **P0** | δ-SPH Particle Shifting | 1 week | ⭐⭐⭐⭐⭐ | DFSPH | **Critical for stability** |
| **P0** | Warm-Starting | 0.5 week | ⭐⭐⭐⭐ | DFSPH | 70-90% fewer iterations |
| **P0** | Matrix-Free Implicit Viscosity | 2-3 weeks | ⭐⭐⭐⭐⭐ | DFSPH | Replaces infeasible CG solver |
| **P1** | Per-Phase Properties | 1 week | ⭐⭐⭐⭐ | None | Foundation for multi-phase |
| **P1** | Validation Suite | 2 weeks | ⭐⭐⭐⭐⭐ | DFSPH | Research credibility |
| **P1** | Hybrid SDF+Akinci Boundaries | 1.5 weeks | ⭐⭐⭐⭐ | None | SDF for density, particles for friction |

### Enhancement Path (Research Excellence)

| Priority | Feature | Effort | Impact | Dependencies | Notes |
|----------|---------|--------|--------|--------------|-------|
| **P2** | Vorticity Confinement | 1 week | ⭐⭐⭐⭐ | DFSPH | **Critical for visual quality** |
| **P2** | δ⁺-SPH Multi-Phase | 1.5 weeks | ⭐⭐⭐⭐⭐ | δ-SPH, Multi-Phase | Interface-aware shifting |
| **P2** | Implicit Air Phase | 2 weeks | ⭐⭐⭐⭐ | Multi-Phase | Splashes, bubbles, foam |
| **P2** | Vorticity-Based Shear Rate | 0.5 week | ⭐⭐⭐ | Viscosity | Less noisy non-Newtonian |
| **P2** | Non-Newtonian (Carreau) | 1.5 weeks | ⭐⭐⭐⭐ | Viscosity | Stable power-law alternative |
| **P2** | Temperature-Viscosity | 1 week | ⭐⭐⭐ | Viscosity | Arrhenius model |
| **P3** | Micropolar SPH | 2 weeks | ⭐⭐⭐⭐ | Vorticity | Particle spin for turbulence |
| **P3** | Turbulence Particles | 1 week | ⭐⭐⭐ | Vorticity | Visual-only enhancement |
| **P3** | VTK Export | 3 days | ⭐⭐⭐ | None | ParaView integration |
| **P3** | Parameter Study | 1 week | ⭐⭐⭐ | Validation | Research automation |

### Optional: Large-Scale Hybrid (Advanced)

| Priority | Feature | Effort | Impact | Dependencies | Notes |
|----------|---------|--------|--------|--------------|-------|
| **P4** | FLIP/APIC Hybrid | 4-6 weeks | ⭐⭐⭐⭐⭐ | Grid infrastructure | Grid + particles for scale |
| **P4** | LOD Multi-Resolution | 2-3 weeks | ⭐⭐⭐⭐ | Particle management | Variable particle sizes |
| **P4** | Sparse Grid (Taichi-style) | 3-4 weeks | ⭐⭐⭐⭐ | Grid infrastructure | VDB-like memory efficiency |

### Timeline Estimate (Revised v2.0)

```
Phase 1: Core Solver (5-6 weeks)
├─ Week 1-3:   PCISPH Implementation + Testing
├─ Week 4:     δ-SPH Particle Shifting (tensile instability fix)
├─ Week 5:     Warm-Starting + Error-Based Early Exit
└─ Week 6:     DFSPH Upgrade (optional, if PCISPH insufficient)

Phase 2: Viscosity Pipeline (3-4 weeks)
├─ Week 7-8:   Matrix-Free Implicit Viscosity (Jacobi)
├─ Week 9:     Vorticity-Based Shear Rate
└─ Week 10:    Non-Newtonian (Carreau model)

Phase 3: Multi-Phase + Air (3-4 weeks)
├─ Week 11:    Per-Phase Properties + δ⁺-SPH Interface
├─ Week 12-13: Implicit Air Phase + Bubble Physics
└─ Week 14:    Interface Tension Matrix

Phase 4: Boundaries + Turbulence (3 weeks)
├─ Week 15:    Hybrid SDF+Akinci Boundaries
├─ Week 16:    Vorticity Confinement
└─ Week 17:    Micropolar SPH (optional) OR Turbulence Particles

Phase 5: Validation & Polish (2-3 weeks)
├─ Week 18-19: Validation Suite (Dam Break, Couette, Poiseuille)
└─ Week 20:    VTK Export + Documentation

═══════════════════════════════════════════════════════════
Total: ~20 weeks for complete research-grade implementation
       ~12 weeks for core functionality (Phases 1-3)
       ~24 weeks if including FLIP/APIC hybrid (Phase 4+)
═══════════════════════════════════════════════════════════
```

---

## Part 4: Technical Specifications (Revised v2.0)

### 4.1 New Buffer Layout

```rust
// Extended particle structure for research-grade simulation
#[repr(C)]
pub struct ResearchParticle {
    // Position-Based (existing)
    pub position: [f32; 4],           // xyz + padding
    pub velocity: [f32; 4],           // xyz + padding
    pub predicted_position: [f32; 4], // xyz + padding
    
    // Density/Pressure (existing)
    pub lambda: f32,
    pub density: f32,
    pub phase: u32,
    pub temperature: f32,
    
    // DFSPH (new)
    pub alpha: f32,                   // Density error factor
    pub kappa: f32,                   // Divergence factor
    pub velocity_divergence: f32,     // ∇·v
    pub density_derivative: f32,      // Dρ/Dt for DFSPH
    pub previous_pressure: f32,       // For warm-starting (NEW)
    
    // Viscosity (new)
    pub viscosity_coefficient: f32,
    pub shear_rate: f32,              // For non-Newtonian
    
    // Particle Shifting - δ-SPH (NEW from expert review)
    pub shift_delta: [f32; 3],        // δr shift vector
    pub is_surface: u32,              // Flag for surface detection
    
    // Vorticity & Turbulence (NEW from expert review)
    pub vorticity: [f32; 3],          // ω = ∇ × v
    pub angular_velocity: [f32; 3],   // For micropolar (optional)
    
    // Multi-Phase (NEW from expert review)
    pub phase_gradient: [f32; 3],     // Interface normal for δ⁺-SPH
    pub is_gas: u32,                  // Air phase flag
    
    // Visualization (existing)
    pub color: [f32; 4],
    
    pub _pad: [f32; 1],               // Alignment padding
}
// Size: 176 bytes (was 80 bytes) - increased for research capabilities
```

**Memory Budget Comparison**:
| Particle Type | Size | Particles @ 1GB VRAM | Use Case |
|--------------|------|---------------------|----------|
| PBD (current) | 80 bytes | 13.4M | Games, real-time |
| Research (v2) | 176 bytes | 6.1M | Research, offline |
| Micropolar (full) | 208 bytes | 5.2M | Advanced research |

### 4.2 New Shader Files

```
shaders/
├── fluid.wgsl                  # Current PBD (keep as fallback)
├── dfsph/
│   ├── predict.wgsl
│   ├── compute_alpha.wgsl
│   ├── solve_density.wgsl
│   ├── compute_kappa.wgsl
│   ├── solve_divergence.wgsl
│   ├── warm_start.wgsl         # NEW: Pressure from previous frame
│   └── integrate.wgsl
├── pcisph/                     # NEW: Alternative solver
│   ├── predict_correct.wgsl
│   ├── density_error.wgsl
│   └── pressure_correction.wgsl
├── viscosity/
│   ├── xsph.wgsl               # Current (keep for games)
│   ├── morris.wgsl             # Explicit Morris
│   ├── implicit_jacobi.wgsl    # NEW: Matrix-free implicit
│   └── shear_rate.wgsl         # NEW: Vorticity-based estimation
├── stability/                  # NEW: Tensile instability fixes
│   ├── particle_shifting.wgsl  # δ-SPH
│   ├── surface_detection.wgsl  # Free-surface identification
│   └── interface_shifting.wgsl # δ⁺-SPH for multi-phase
├── vorticity/                  # NEW: Turbulence enrichment
│   ├── compute_curl.wgsl
│   ├── confinement.wgsl
│   └── micropolar.wgsl         # Particle spin (optional)
├── multiphase/
│   ├── interface_tension.wgsl
│   ├── phase_gradient.wgsl     # NEW: Interface detection
│   ├── air_spawning.wgsl       # NEW: Implicit air phase
│   └── phase_mixing.wgsl
├── boundary/                   # NEW: Enhanced boundaries
│   ├── akinci_particles.wgsl
│   └── sdf_density.wgsl        # NEW: SDF-based contribution
└── validation/
    └── metrics.wgsl            # Compute conservation errors
```

### 4.3 Configuration API (Revised)

```rust
pub struct ResearchFluidConfig {
    // Solver selection
    pub solver: SolverType,  // PBD, DFSPH, IISPH
    
    // Accuracy
    pub max_density_error: f32,      // Target: 0.001 (0.1%)
    pub max_iterations: u32,         // 100 for research
    pub min_iterations: u32,         // 3 for games
    
    // Phases
    pub phases: Vec<FluidPhase>,
    pub interface_tensions: InterfaceTensionMatrix,
    
    // Viscosity
    pub viscosity_solver: ViscositySolver,  // XSPH, Morris, Implicit
    pub enable_non_newtonian: bool,
    pub enable_temperature_viscosity: bool,
    pub shear_rate_method: ShearRateMethod, // NEW: Tensor vs Vorticity
    
    // Stability (NEW from expert review)
    pub enable_particle_shifting: bool,     // δ-SPH
    pub shifting_method: ShiftingMethod,    // Standard, InterfaceAware
    pub enable_warm_start: bool,            // Reuse previous pressure
    
    // Turbulence (NEW from expert review)
    pub enable_vorticity_confinement: bool,
    pub vorticity_epsilon: f32,             // Confinement strength
    pub enable_micropolar: bool,            // Particle spin
    
    // Air Phase (NEW from expert review)
    pub enable_implicit_air: bool,
    pub air_spawn_threshold: f32,           // Velocity for splash spawn
    pub max_air_particles: u32,
    
    // Boundaries
    pub boundary_method: BoundaryMethod,    // SDF, Akinci, Hybrid
    
    // Validation
    pub export_metrics: bool,
    pub metric_interval: u32,               // Frames between snapshots
}

pub enum SolverType {
    PBD,     // Fast, visual (games)
    PCISPH,  // NEW: Balanced, simpler than DFSPH
    DFSPH,   // Accurate (AAA games, pre-viz)
    IISPH,   // Most stable (research, VFX)
}

pub enum ViscositySolver {
    XSPH,           // Fast, artificial
    Morris,         // Physically-based, explicit
    ImplicitJacobi, // NEW: Matrix-free implicit (recommended)
}

pub enum ShiftingMethod {
    None,
    StandardDelta,      // δ-SPH (single phase)
    InterfaceAware,     // δ⁺-SPH (multi-phase)
}

pub enum ShearRateMethod {
    StrainTensor,       // Accurate but noisy
    VorticityBased,     // Smoother (recommended)
    Blended,            // 70% vorticity + 30% strain
}

pub enum BoundaryMethod {
    AkinciOnly,         // Traditional particle sampling
    SDFOnly,            // Fast but less accurate friction
    Hybrid,             // SDF density + Akinci friction (recommended)
}
```

---

## Part 5: Performance Considerations (Revised v2.0)

### 5.1 Solver Performance Comparison

| Solver | Typical Iterations | Time/Frame (100k particles) | Memory | Stability |
|--------|-------------------|----------------------------|--------|-----------|
| PBD | 3-5 | ~2ms | 80 bytes | Good |
| PCISPH | 3-8 | ~3.5ms | 128 bytes | Better |
| DFSPH | 2-3 + 1-2 | ~4.5ms | 144 bytes | Excellent |
| DFSPH + δ-SPH | 2-3 + 1-2 + 1 | ~5ms | 176 bytes | Best |
| IISPH | 10-50 Jacobi | ~8ms | 144 bytes | Excellent |

> ⚠️ **Note**: With warm-starting, DFSPH often converges in 1-2 iterations instead of 5-10, making it competitive with PBD for well-behaved scenarios.

**Realistic Performance Targets (Revised)**:
| Quality Tier | Particle Count | Target FPS | Solver | Features |
|--------------|---------------|------------|--------|----------|
| Low (Mobile) | 50-100k | 60 | PBD | XSPH only |
| Medium (Console) | 100-200k | 60 | PCISPH | Morris, basic shifting |
| High (PC) | 200-350k | 60 | DFSPH | Full δ-SPH, vorticity |
| Ultra (PC) | 350-500k | 30 | DFSPH | Multi-phase, micropolar |
| Research | 500k-1M | Offline | DFSPH/IISPH | All features + VTK export |

### 5.2 Memory Budget (Revised)

| Component | Size (100k particles) | Notes |
|-----------|----------------------|-------|
| Particles (PBD) | 8 MB | Current 80-byte struct |
| Particles (Research) | 17.6 MB | 176-byte struct with all features |
| Grid | 8 MB | 128³ × 4 bytes |
| Boundaries | ~2 MB | Surface samples |
| Velocity Buffers (implicit) | 2.4 MB | Ping-pong for Jacobi |
| Vorticity Buffer | 1.2 MB | 12 bytes × 100k |
| **Total (Research Mode)** | ~31 MB | vs ~20 MB for PBD |

### 5.3 GPU Dispatch Strategy

```rust
// Adaptive workgroup sizing (existing infrastructure)
let workgroup_size = self.workgroup_config.preferred_size();  // 64-256

// DFSPH specific: smaller groups for divergence solve
let divergence_workgroup = min(workgroup_size, 128);  // More L1 cache hits

// Warm-starting pass: very fast, large workgroups OK
let warmstart_workgroup = workgroup_size;

// Particle shifting: moderate compute, standard sizing
let shifting_workgroup = workgroup_size;
```

### 5.4 Warm-Starting Impact (NEW)

| Scenario | Without Warm-Start | With Warm-Start | Speedup |
|----------|-------------------|-----------------|---------|
| Steady flow | 8 iterations | 1-2 iterations | 4-8× |
| Moderate splash | 12 iterations | 3-4 iterations | 3-4× |
| High-energy impact | 20 iterations | 8-12 iterations | 1.5-2.5× |

**Key Insight**: Warm-starting is most effective for steady or slowly-changing flows. It's essential for interactive rates.

---

## Part 6: Testing Strategy

### 6.1 Unit Tests

```rust
#[test]
fn test_morris_viscosity_couette_flow() {
    // Setup: Two parallel plates, bottom moving
    // Expected: Linear velocity profile v(y) = V * y/H
    // Verify: RMSE < 1% of analytical solution
}

#[test]
fn test_dfsph_hydrostatic_pressure() {
    // Setup: Column of water at rest
    // Expected: P(y) = ρg(H-y)
    // Verify: Max pressure error < 0.5%
}

#[test]
fn test_multiphase_separation() {
    // Setup: Oil and water mixture
    // Expected: Oil floats to top within 5 seconds
    // Verify: 95% of oil particles above 95% of water particles
}
```

### 6.2 Validation Benchmarks

```rust
pub struct DamBreakBenchmark {
    // Martin & Moyce 1952 experimental data
    pub experimental_front_positions: Vec<(f32, f32)>,  // (time, x_position)
}

impl DamBreakBenchmark {
    pub fn compare(&self, simulation_data: &[(f32, f32)]) -> ValidationResult {
        // Compute RMSE, max error, correlation coefficient
    }
}
```

---

## Part 7: Documentation & References

### Key Research Papers

1. **DFSPH**: Bender & Koschier, "Divergence-Free Smoothed Particle Hydrodynamics" (2015, 2017)
2. **PCISPH**: Solenthaler & Pajarola, "Predictive-Corrective Incompressible SPH" (2009)
3. **IISPH**: Ihmsen et al., "Implicit Incompressible SPH" (2014)
4. **Morris Viscosity**: Morris et al., "Modeling Low Reynolds Number Incompressible Flows" (1997)
5. **Weiler Viscosity**: Weiler et al., "Projective Fluids" (2016), Implicit viscosity (2018)
6. **δ-SPH**: Marrone et al., "δ-SPH Model for Simulating Violent Impact Flows" (2011)
7. **δ⁺-SPH**: Sun et al., "A Consistent Approach to Particle Shifting" (2017)
8. **Micropolar SPH**: Bender et al., "Micropolar Smoothed Particle Hydrodynamics" (2017)
9. **Akinci Surface Tension**: Akinci et al., "Versatile Surface Tension and Adhesion" (2013)
10. **Akinci Boundaries**: Akinci et al., "Versatile Rigid-Fluid Coupling" (2012)
11. **Vorticity Confinement**: Fedkiw et al., "Visual Simulation of Smoke" (2001)
12. **SPH Tutorial**: Koschier et al., Eurographics 2019/2022 (comprehensive reference)

### Open Source References

- **SPlisHSPlasH**: RWTH Aachen's reference SPH implementation
  - GitHub: InteractiveComputerGraphics/SPlisHSPlasH
  - Implements DFSPH, IISPH, all viscosity models, particle shifting
  - **Gold standard for research validation**
  
- **Taichi SPH Examples**: GPU-accelerated SPH with sparse data structures
  - GitHub: taichi-dev/taichi
  - Excellent for GPU optimization patterns
  
- **Salva3d**: Rust SPH library
  - GitHub: dimforge/salva
  - DFSPH solver, boundary handling

### Recommended Reading Order

1. SPH Tutorial (foundations)
2. Solenthaler 2009 (PCISPH - simpler starting point)
3. Bender & Koschier 2017 (DFSPH details)
4. Marrone 2011 + Sun 2017 (δ-SPH / δ⁺-SPH for stability)
5. Weiler 2018 (implicit viscosity)
6. Bender 2017 (micropolar for turbulence)
7. Morris 1997 (viscosity fundamentals)
8. Akinci 2013 (surface tension)
9. Ihmsen 2014 (IISPH for comparison)

---

## Part 8: Expert Review Summary (v2.0 Additions)

### Issues Addressed from Expert Review

| Issue | Original Plan | v2.0 Fix |
|-------|--------------|----------|
| Overly optimistic 1M+ particles | Implied achievable | Added realistic tier table (50k-1M with LOD) |
| Full CG solver infeasible on GPU | "Conjugate gradient solver" | Matrix-free Jacobi (O(n) memory, parallel) |
| Missing tensile instability fix | Not mentioned | Added δ-SPH particle shifting (Phase 1.3) |
| Suboptimal multi-phase | Basic interface tension | Added δ⁺-SPH interface-aware shifting |
| Viscosity after integration | Pipeline order wrong | Moved to BEFORE/WITHIN pressure solve |
| Boundary resampling cost | Full Akinci | Added Hybrid SDF+Akinci option |
| Non-Newtonian shear noise | Strain tensor only | Added vorticity-based shear estimation |
| Limited temperature model | Viscosity only | Acknowledged, deferred to future work |
| Lack of turbulence/vorticity | Only basic confinement | Added full Phase 5: Vorticity & Micropolar |
| Fixed iteration assumptions | 100 iterations | Added warm-starting + adaptive convergence |

### New Features Added in v2.0

1. **PCISPH Solver** - Simpler alternative to DFSPH
2. **δ-SPH Particle Shifting** - Fixes tensile instability
3. **δ⁺-SPH Multi-Phase** - Interface-aware shifting
4. **Warm-Starting** - 70-90% fewer solver iterations
5. **Matrix-Free Implicit Viscosity** - GPU-efficient high-viscosity
6. **Vorticity Confinement** - Re-inject lost small-scale vortices
7. **Micropolar SPH** - Particle spin for fine turbulence
8. **Turbulence Particles** - Visual-only detail enhancement
9. **Implicit Air Phase** - Splashes, bubbles, foam
10. **Hybrid SDF+Akinci Boundaries** - Efficient complex geometry
11. **Vorticity-Based Shear Rate** - Smoother non-Newtonian

### Deferred to Future Work

- **Full Heat Diffusion/Advection**: Temperature affects viscosity but doesn't transport yet
- **FLIP/APIC Hybrid**: Listed as optional P4 for truly large-scale (1M+)
- **Sparse Grid Structures**: Taichi-style VDB for memory efficiency
- **Multi-Resolution LOD**: Variable particle sizes in same simulation

---

## Conclusion

This enhanced plan (v2.0) addresses all critical issues identified by expert review and provides a clear roadmap to transform AstraWeave Fluids from a **production game fluid system (Grade B)** to a **research-grade simulation platform (Grade A+)**. The modular approach allows:

1. **Incremental adoption**: Keep PBD as fallback, add PCISPH/DFSPH as options
2. **Performance scaling**: LOD between PBD/DFSPH based on importance, warm-starting for efficiency
3. **Stability guarantees**: δ-SPH prevents tensile instability, vorticity confinement prevents over-damping
4. **Research validation**: Standardized tests against experimental data, VTK export
5. **Industry flexibility**: Support water, oils, honey, and exotic fluids with proper viscosity handling

**Estimated Total Effort**: 
- Core functionality (Phases 1-3): 12-14 weeks
- Full research-grade (Phases 1-6): 18-22 weeks
- Including FLIP/APIC hybrid: 24-28 weeks

**Key Success Metrics**:
- Density error: <0.1% (vs current ~5%)
- Divergence error: <0.01%
- Couette flow accuracy: <1% RMSE
- Dam break correlation: >0.95 R²
- Viscosity range: 0.001 - 10+ Pa·s (water to honey)
- Tensile instability: Eliminated via δ-SPH
- Visual turbulence: Enhanced via vorticity confinement

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | January 2025 | Initial research-grade enhancement plan |
| 2.0 | January 2026 | **Major revision** based on expert review: Added PCISPH, δ-SPH/δ⁺-SPH particle shifting, matrix-free implicit viscosity, vorticity confinement, micropolar SPH, implicit air phase, hybrid boundaries, warm-starting. Revised performance targets to be realistic. Fixed viscosity pipeline ordering. |

---

*Document generated for AstraWeave AI-Native Gaming Engine*
*Version 2.0 - January 2026 (Revised with Expert Feedback)*
