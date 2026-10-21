# Phase PBR-E: Advanced Materials Design Document

**Status**: In Progress  
**Date**: October 2025  
**Phase**: PBR-E (Advanced Materials)  

---

## Overview

Phase PBR-E extends the Cook-Torrance BRDF foundation from Phase PBR-D with 5 advanced material features matching UE5/Unity HDRP quality:

1. **Clearcoat** - 2nd specular lobe for car paint, lacquer, varnish
2. **Anisotropy** - Directional highlights for brushed metal, hair, fabric
3. **Subsurface Scattering (SSS)** - Translucency for skin, wax, marble
4. **Sheen** - Retroreflection for velvet, satin, cloth
5. **Transmission** - Refraction for glass, water, ice, gemstones

---

## Physical Theory

### 1. Clearcoat Layer
**Concept**: Thin transparent coating over base material (car paint = metallic base + clear coat)

**Physics**:
- 2nd specular lobe with fixed IOR = 1.5 (polyurethane/acrylic)
- GGX distribution with separate roughness parameter
- Fresnel at air-coating interface (~4% at normal incidence)
- Energy conservation: base layer receives (1 - F_clearcoat) of light

**Parameters**:
- `clearcoat_strength`: [0,1] - blend between base-only and coated
- `clearcoat_roughness`: [0,1] - independent roughness for coating
- Optional: `clearcoat_normal_map` for orange-peel effect

**BRDF Formula**:
```
specular_total = specular_base × (1 - F_coat) + specular_coat
where:
  F_coat = fresnel_schlick(N·V, F0=0.04) × clearcoat_strength
  specular_coat = D_ggx(coat_roughness) × G_smith(coat_roughness) × F_coat / 4(N·V)(N·L)
```

**Reference**: Burley 2012 (Physically-Based Shading at Disney), Karis 2013 (UE4 PBR)

---

### 2. Anisotropic Reflections
**Concept**: Elliptical highlight shape from directional microstructure (brushed metal grooves)

**Physics**:
- Anisotropic GGX replaces isotropic version
- Separate roughness along tangent (α_t) and bitangent (α_b)
- Requires tangent space (T, B, N) for orientation
- Rotation parameter controls groove direction

**Parameters**:
- `anisotropy_strength`: [-1, 1] - negative = perpendicular grooves, positive = parallel
- `anisotropy_rotation`: [0, 2π] - rotate groove direction
- Derived: α_t = α × (1 + anisotropy), α_b = α × (1 - anisotropy)

**BRDF Formula (Burley 2012)**:
```
D_aniso(H) = 1 / (π × α_t × α_b × denom²)
where:
  denom = (T·H / α_t)² + (B·H / α_b)² + (N·H)²
```

**Tangent Generation**:
- From UV derivatives: dPdu (tangent direction)
- Gram-Schmidt orthogonalization: T = normalize(dPdu - (dPdu·N)×N)
- Bitangent: B = cross(N, T)

**Reference**: Burley 2012, Kulla & Conty 2017 (Revisiting Physically Based Shading)

---

### 3. Subsurface Scattering (SSS)
**Concept**: Light penetrates surface, scatters internally, exits nearby (skin glow)

**Physics**:
- Approximation: Diffuse term replaced with wrapped diffuse or Burley diffusion
- Full SSS = Monte Carlo path tracing (too expensive for realtime)
- Realtime: Screen-space blur or separable Gaussian approximation

**Parameters**:
- `subsurface_color`: RGB - tint of scattered light (e.g., reddish for skin)
- `subsurface_radius`: float - average scattering distance (mm)
- `subsurface_scale`: [0,1] - blend between Lambertian and SSS
- Optional: `thickness_map` - modulate scattering by surface thickness

**Approximation (Burley 2015)**:
```
diffuse_sss = subsurface_color × (A × wrap(N·L, 0.5) + B × wrap(N·L, -0.5))
where:
  wrap(x, w) = (x + w) / (1 + w)
  A, B = precomputed profile coefficients
```

**Screen-Space SSS** (higher quality, optional):
- Render thickness map (back-face depth - front-face depth)
- Separable Gaussian blur weighted by thickness and view-space distance
- 3-5 samples per axis for performance

**Reference**: Burley 2015 (Extending Disney BRDF), Jimenez 2015 (Separable SSS)

---

### 4. Sheen (Fabric Retroreflection)
**Concept**: Velvet/satin appear brighter at grazing angles (opposite of Fresnel)

**Physics**:
- Charlie sheen BRDF (inverted Gaussian)
- Peaks at grazing angles (90°), fades at normal incidence
- Energy-conserving: reduces diffuse lobe proportionally

**Parameters**:
- `sheen_color`: RGB - tint of retroreflected light (often white or fabric color)
- `sheen_roughness`: [0,1] - controls falloff sharpness

**BRDF Formula (Estevez & Kulla 2017)**:
```
sheen(V, L, H) = sheen_color × D_charlie(roughness) × V_ashikhmin(N·V, N·L)
where:
  D_charlie(α) = (2 + 1/α) × (1 - cos²θ)^(1/(2α)) / (2π)
  V_ashikhmin = 1 / (4 × (N·L + N·V - N·L × N·V))
```

**Energy Conservation**:
```
diffuse_with_sheen = diffuse × (1 - max(sheen_color)) + sheen
```

**Reference**: Estevez & Kulla 2017 (Production Friendly Sheen BRDF)

---

### 5. Transmission (Glass/Refraction)
**Concept**: Light refracts through transparent materials with Beer-Lambert attenuation

**Physics**:
- BTDF (Bidirectional Transmittance Distribution Function) instead of BRDF
- Snell's law: η₁ sin θ₁ = η₂ sin θ₂
- Beer-Lambert attenuation: I(d) = I₀ × e^(-σ×d) or simplified I(d) = I₀ × attenuation_color^(d / attenuation_distance)
- Fresnel determines reflection vs transmission ratio

**Parameters**:
- `transmission_factor`: [0,1] - opacity (0 = opaque, 1 = fully transparent)
- `ior`: float - index of refraction (glass=1.5, water=1.33, diamond=2.42)
- `attenuation_color`: RGB - absorption tint (green glass, amber, etc.)
- `attenuation_distance`: float - distance for 50% absorption
- Optional: `thin_film_thickness`, `thin_film_ior` for iridescence

**Approximation Methods**:
1. **Screen-Space Refraction** (fast):
   - Offset UV by refracted direction × roughness
   - Sample scene color buffer at offset UV
   - Apply Beer-Lambert attenuation based on thickness

2. **Environment Refraction** (for non-planar):
   - Refract view ray using Snell's law
   - Sample environment map along refracted direction
   - Apply attenuation

**BTDF Formula (Microfacet Transmission, Walter et al. 2007)**:
```
transmission = (1 - F) × D × G × |V·H| × |L·H| / |N·V| × |N·L| × |η₁(V·H) + η₂(L·H)|²
```

**Reference**: Walter et al. 2007 (Microfacet Models for Refraction), Drobot 2017 (Physically Based Sky)

---

## Material Schema Extension

### MaterialGpu Structure (WGSL)
```wgsl
struct MaterialGpu {
    // Base PBR (Phase PBR-D)
    albedo_index: u32,
    normal_index: u32,
    orm_index: u32,      // Occlusion, Roughness, Metallic
    flags: u32,          // Bitfield for features
    
    // Base factors
    base_color_factor: vec4<f32>,
    metallic_factor: f32,
    roughness_factor: f32,
    occlusion_strength: f32,
    emissive_factor: vec3<f32>,
    
    // Clearcoat
    clearcoat_strength: f32,
    clearcoat_roughness: f32,
    clearcoat_normal_index: u32,
    _pad0: u32,
    
    // Anisotropy
    anisotropy_strength: f32,
    anisotropy_rotation: f32,
    _pad1: vec2<f32>,
    
    // Subsurface
    subsurface_color: vec3<f32>,
    subsurface_scale: f32,
    subsurface_radius: f32,
    thickness_index: u32,
    _pad2: vec2<f32>,
    
    // Sheen
    sheen_color: vec3<f32>,
    sheen_roughness: f32,
    
    // Transmission
    transmission_factor: f32,
    ior: f32,
    attenuation_color: vec3<f32>,
    attenuation_distance: f32,
    _pad3: vec2<f32>,
}
```

**Size**: 256 bytes (16-byte alignment for UBO/SSBO)

### materials.toml Schema Extension
```toml
[[materials]]
name = "car_paint_red"
albedo = "car_red_albedo.ktx2"
normal = "car_normal.ktx2"
orm = "car_orm.ktx2"
metallic_factor = 0.9
roughness_factor = 0.3

# Clearcoat layer (car paint)
clearcoat_strength = 1.0
clearcoat_roughness = 0.05
clearcoat_normal = "orange_peel.ktx2"  # Optional

[[materials]]
name = "brushed_aluminum"
albedo = "metal_albedo.ktx2"
normal = "metal_normal.ktx2"
orm = "metal_orm.ktx2"
metallic_factor = 1.0
roughness_factor = 0.4

# Anisotropic reflections
anisotropy_strength = 0.8
anisotropy_rotation = 0.0  # radians

[[materials]]
name = "skin_caucasian"
albedo = "skin_albedo.ktx2"
normal = "skin_normal.ktx2"
orm = "skin_orm.ktx2"

# Subsurface scattering
subsurface_color = [0.9, 0.3, 0.3]  # Reddish
subsurface_scale = 0.7
subsurface_radius = 1.5  # mm
thickness_map = "skin_thickness.ktx2"

[[materials]]
name = "red_velvet"
albedo = "velvet_albedo.ktx2"
normal = "velvet_normal.ktx2"
orm = "velvet_orm.ktx2"

# Sheen for fabric
sheen_color = [1.0, 1.0, 1.0]
sheen_roughness = 0.3

[[materials]]
name = "clear_glass"
albedo = "glass_albedo.ktx2"
normal = "glass_normal.ktx2"
orm = "glass_orm.ktx2"
metallic_factor = 0.0
roughness_factor = 0.05

# Transmission
transmission_factor = 0.95
ior = 1.5
attenuation_color = [0.9, 1.0, 0.9]  # Slight green tint
attenuation_distance = 10.0  # cm
```

---

## WGSL Function Signatures

### Clearcoat
```wgsl
fn clearcoat_distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32;
fn clearcoat_geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32;
fn clearcoat_fresnel(cos_theta: f32) -> f32;  // Fixed F0 = 0.04 (IOR 1.5)
fn evaluate_clearcoat(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, H: vec3<f32>, material: MaterialGpu) -> vec3<f32>;
```

### Anisotropy
```wgsl
fn compute_tangent_basis(N: vec3<f32>, dpdu: vec3<f32>) -> TangentBasis;
fn rotate_tangent_basis(T: vec3<f32>, B: vec3<f32>, rotation: f32) -> TangentBasis;
fn distribution_ggx_anisotropic(T: vec3<f32>, B: vec3<f32>, N: vec3<f32>, H: vec3<f32>, alpha_t: f32, alpha_b: f32) -> f32;
fn geometry_smith_anisotropic(T: vec3<f32>, B: vec3<f32>, N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, alpha_t: f32, alpha_b: f32) -> f32;
fn evaluate_anisotropic_specular(T: vec3<f32>, B: vec3<f32>, N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, H: vec3<f32>, F0: vec3<f32>, material: MaterialGpu) -> vec3<f32>;
```

### Subsurface Scattering
```wgsl
fn wrap_diffuse(n_dot_l: f32, wrap: f32) -> f32;
fn burley_diffusion_profile(n_dot_l: f32, subsurface_color: vec3<f32>) -> vec3<f32>;
fn evaluate_subsurface(N: vec3<f32>, L: vec3<f32>, material: MaterialGpu) -> vec3<f32>;
// Screen-space SSS (optional, post-process)
fn separable_sss_blur(thickness: f32, radius: f32, direction: vec2<f32>) -> vec3<f32>;
```

### Sheen
```wgsl
fn distribution_charlie(roughness: f32, n_dot_h: f32) -> f32;
fn visibility_ashikhmin(n_dot_v: f32, n_dot_l: f32) -> f32;
fn evaluate_sheen(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, H: vec3<f32>, material: MaterialGpu) -> vec3<f32>;
```

### Transmission
```wgsl
fn fresnel_dielectric(cos_theta_i: f32, eta: f32) -> f32;  // Exact Fresnel
fn refract_ray(I: vec3<f32>, N: vec3<f32>, eta: f32) -> vec3<f32>;
fn beer_lambert_attenuation(distance: f32, attenuation_color: vec3<f32>, attenuation_distance: f32) -> vec3<f32>;
fn evaluate_transmission(N: vec3<f32>, V: vec3<f32>, material: MaterialGpu, env_sample: vec3<f32>, thickness: f32) -> vec3<f32>;
```

### Combined Evaluation
```wgsl
fn evaluate_pbr_advanced(
    N: vec3<f32>,
    V: vec3<f32>,
    L: vec3<f32>,
    material: MaterialGpu,
    light_color: vec3<f32>,
    env_sample: vec3<f32>,
    thickness: f32
) -> vec3<f32>;
```

---

## Energy Conservation Strategy

Multi-lobe materials require careful energy conservation to avoid over-bright rendering:

### Lobe Blending Order
1. **Transmission** - Splits light into reflected/transmitted (Fresnel ratio)
2. **Clearcoat** - Attenuates base layer by (1 - F_coat)
3. **Base Specular** - Standard Cook-Torrance
4. **Sheen** - Reduces diffuse proportionally
5. **Diffuse/SSS** - Remaining energy after specular/sheen

### Formula
```wgsl
// 1. Transmission splits light
let F_transmission = fresnel_dielectric(N·V, material.ior);
let transmitted = (1.0 - F_transmission) × material.transmission_factor × transmission_lobe;
let reflected_energy = 1.0 - material.transmission_factor × (1.0 - F_transmission);

// 2. Clearcoat attenuates base
let F_coat = clearcoat_fresnel(N·V) × material.clearcoat_strength;
let clearcoat_lobe = evaluate_clearcoat(...);
let base_energy = reflected_energy × (1.0 - F_coat);

// 3. Base specular (Cook-Torrance from PBR-D)
let F_base = fresnel_schlick(H·V, F0);
let base_specular = base_energy × evaluate_cook_torrance(...);

// 4. Sheen reduces diffuse
let sheen_lobe = evaluate_sheen(...);
let sheen_max = max(max(material.sheen_color.r, material.sheen_color.g), material.sheen_color.b);
let diffuse_energy = base_energy × (1.0 - F_base) × (1.0 - metallic) × (1.0 - sheen_max);

// 5. Diffuse/SSS
let diffuse_lobe = mix(lambert_diffuse, evaluate_subsurface(...), material.subsurface_scale);

// Combine all lobes
let final_color = transmitted + clearcoat_lobe + base_specular + sheen_lobe + diffuse_lobe × diffuse_energy;
```

---

## Performance Budget

### ALU Cost Estimates (per pixel)
| Feature | ALU Ops | Notes |
|---------|---------|-------|
| Base PBR (PBR-D) | 150-200 | GGX + Smith + Fresnel + IBL |
| + Clearcoat | +80-100 | 2nd GGX lobe |
| + Anisotropy | +40-60 | Elliptical distribution |
| + SSS (simple) | +20-30 | Wrapped diffuse |
| + SSS (screen-space) | +200-400 | Separable blur (post-process) |
| + Sheen | +30-40 | Charlie distribution |
| + Transmission | +50-80 | Refraction + Beer-Lambert |
| **Total (all features)** | **370-510** | Without screen-space SSS |
| **Total (with SS-SSS)** | **570-910** | With screen-space SSS |

### Optimization Strategies
1. **Feature Flags**: Use `material.flags` bitfield to skip disabled features
2. **Material Sorting**: Batch by feature set to reduce branching
3. **LOD**: Disable expensive features at distance (clearcoat only, no SSS)
4. **Quality Presets**: Low/Medium/High toggle screen-space SSS, sample counts

---

## Testing Strategy

### Unit Tests (`test_pbr_advanced.rs`)
1. **Clearcoat Energy Conservation**: `specular_total ≤ 1.0` for all angles
2. **Anisotropic NDF**: Validate elliptical shape, tangent/bitangent correctness
3. **SSS Profiles**: Validate wrapped diffuse non-negativity, profile sum = 1.0
4. **Sheen Retroreflection**: Verify grazing angle > normal incidence
5. **Transmission Fresnel**: Validate reflected + transmitted = 1.0

### Visual Validation Scenes
1. **Clearcoat Sphere Grid**: Strength 0→1 (X), Roughness 0→1 (Y)
2. **Anisotropic Sphere Grid**: Strength -1→1 (X), Rotation 0→2π (Y)
3. **SSS Sphere Grid**: Subsurface scale 0→1 (X), Radius 0→5mm (Y)
4. **Sheen Sphere Grid**: Sheen roughness 0→1 (X), Color intensity (Y)
5. **Transmission Sphere Grid**: IOR 1.0→2.5 (X), Attenuation 0→1 (Y)

### Performance Benchmarks
- Measure per-feature ALU cost via GPU profiler (RenderDoc/PIX)
- Compare frame times: base PBR vs +clearcoat vs +all features
- Validate 60 FPS at 1080p with 1000 instances (mixed materials)

---

## Implementation Phases

### Phase PBR-E-1: Clearcoat (1 week)
- [ ] Define MaterialGpuExtended with clearcoat parameters
- [ ] Implement clearcoat BRDF functions in pbr_lib.wgsl
- [ ] Add clearcoat evaluation to evaluate_pbr_advanced()
- [ ] Create car_paint test materials
- [ ] Unit tests for clearcoat energy conservation

### Phase PBR-E-2: Anisotropy (1 week)
- [ ] Implement tangent basis computation and rotation
- [ ] Add anisotropic GGX distribution and geometry functions
- [ ] Integrate into evaluate_pbr_advanced()
- [ ] Create brushed_metal test materials
- [ ] Unit tests for anisotropic NDF

### Phase PBR-E-3: SSS (1.5 weeks)
- [ ] Implement Burley diffusion profile (wrapped diffuse)
- [ ] Add screen-space SSS blur pass (optional, post-process)
- [ ] Integrate into diffuse lobe with blend factor
- [ ] Create skin/wax test materials
- [ ] Unit tests for SSS profiles

### Phase PBR-E-4: Sheen (3-4 days)
- [ ] Implement Charlie distribution and Ashikhmin visibility
- [ ] Integrate sheen lobe with diffuse energy conservation
- [ ] Create velvet/satin test materials
- [ ] Unit tests for sheen retroreflection

### Phase PBR-E-5: Transmission (1.5 weeks)
- [ ] Implement refraction and Beer-Lambert attenuation
- [ ] Add screen-space or environment refraction sampling
- [ ] Integrate transmission with Fresnel splitting
- [ ] Create glass/water test materials
- [ ] Unit tests for transmission Fresnel

### Phase PBR-E-6: Integration & Optimization (1 week)
- [ ] Unified evaluate_pbr_advanced() with all lobes
- [ ] Feature flag system for runtime toggling
- [ ] Material batching by feature set
- [ ] Performance profiling and optimization
- [ ] Visual validation scenes

**Total Estimated Time**: 6-7 weeks

---

## Acceptance Criteria

### Functional
- ✅ All 5 advanced features compile and render correctly
- ✅ Energy conservation verified for multi-lobe materials
- ✅ Feature flags enable/disable individual lobes
- ✅ Material batching reduces draw calls by feature set

### Quality
- ✅ Visual quality matches UE5/Unity HDRP reference images
- ✅ No visual artifacts (fireflies, energy gain, over-bright)
- ✅ Smooth parameter transitions (0→1 blends cleanly)

### Performance
- ✅ 60 FPS at 1080p with 1000 instances (mixed materials)
- ✅ ALU cost within budget (370-510 ops without SS-SSS)
- ✅ Material sorting reduces GPU state changes

### Testing
- ✅ 20+ unit tests covering all BRDF components
- ✅ Visual validation scenes render correctly
- ✅ Performance benchmarks meet targets

---

## References

1. Burley, B. (2012). "Physically-Based Shading at Disney." SIGGRAPH Course.
2. Burley, B. (2015). "Extending the Disney BRDF to a BSDF with Integrated Subsurface Scattering." SIGGRAPH Course.
3. Karis, B. (2013). "Real Shading in Unreal Engine 4." SIGGRAPH Course.
4. Walter, B. et al. (2007). "Microfacet Models for Refraction through Rough Surfaces." EGSR.
5. Estevez, A. & Kulla, C. (2017). "Production Friendly Microfacet Sheen BRDF." SIGGRAPH.
6. Jimenez, J. et al. (2015). "Separable Subsurface Scattering." GPU Pro 6.
7. Kulla, C. & Conty, A. (2017). "Revisiting Physically Based Shading at Imageworks." SIGGRAPH Course.
8. Heitz, E. (2014). "Understanding the Masking-Shadowing Function in Microfacet-Based BRDFs." JCGT.

---

**Document Version**: 1.0  
**Status**: Design Complete - Ready for Implementation  
**Next Steps**: Begin Phase PBR-E-1 (Clearcoat implementation)
