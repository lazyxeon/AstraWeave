//! Climate map generation for biome assignment.
//!
//! Phase 1.6-F.4.B.3.D.1 extends this module with a real-units climate-field
//! architecture (per-vertex temperature in °C, moisture in mm/yr, continentalness
//! in [0,1]) driven by a `WorldArchetype` envelope, latitude/elevation/water-distance
//! modulators, and a low-frequency continentalness noise mirroring `TerrainNoise`'s
//! continental modulation. The legacy `sample_climate`/`sample_temperature`/
//! `sample_moisture` methods returning normalized `[0,1]` values are preserved for
//! existing consumers (`biome_detector`, `biome_transition`, renderer overlay,
//! benchmarks, tests) until F.4.B.3.D.3 replaces them with per-biome parameter
//! lookup. New code should use [`ClimateMap::sample`] returning [`ClimateSample`].

use crate::ChunkId;
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};

/// Phase 1.6-F.4.B.3.D.1: half-extent (in world units) used as the latitude
/// normalization denominator. World Z divided by this constant produces a
/// `[-1, 1]` latitude factor used by the temperature latitude modulator.
///
/// Calibrated for Target B world scale: 21 chunks of 512 WU per side =
/// 10752 WU total extent, half-extent 5376 WU. At Target B, latitude=0 at
/// world center, latitude=±1 at the world edges. Target C will need this
/// recalibrated; default is parameterized via `ClimateConfig` so callers
/// can override.
pub const TARGET_B_LATITUDE_HALF_EXTENT_WU: f32 = 5376.0;

/// Phase 1.6-F.4.B.3.D.1: standard atmospheric lapse rate (-6.5°C per 1000m
/// elevation). Real-world value used unchanged.
pub const ATMOSPHERIC_LAPSE_RATE_C_PER_M: f32 = -0.0065;

/// Phase 1.6-F.4.B.3.D.1: documented bounds on real-units climate sample
/// values. Worlds beyond these ranges fall outside Earth analogs; bounds
/// chosen to span hot-desert (~+40°C, ~50mm) through polar (-30°C, ~200mm)
/// to tropical-rainforest (~+27°C, ~4000mm).
pub const TEMPERATURE_C_MIN: f32 = -30.0;
pub const TEMPERATURE_C_MAX: f32 = 40.0;
pub const MOISTURE_MM_MIN: f32 = 0.0;
pub const MOISTURE_MM_MAX: f32 = 4000.0;

/// Phase 1.6-F.4.B.3.D.1: a single per-vertex climate sample in real-world
/// units. Drives Whittaker biome lookup (D.2) and per-biome parameter
/// blending (D.3, D.4).
///
/// Phase 1.X-F.1.B extends with `erosion` and `weirdness` fields for the
/// regional archetype variation campaign's per-archetype shape splines
/// (campaign doc §2.2). PV is derived at sample time via [`Self::pv`].
///
/// Range invariants (enforced by `ClimateMap::sample`):
/// - `temperature_c` ∈ `[TEMPERATURE_C_MIN, TEMPERATURE_C_MAX]`.
/// - `moisture_mm`  ∈ `[MOISTURE_MM_MIN, MOISTURE_MM_MAX]`.
/// - `continentalness` ∈ `[0.0, 1.0]`.
/// - `erosion` ∈ `[-1.0, 1.0]`.
/// - `weirdness` ∈ `[-1.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClimateSample {
    /// Mean annual temperature in degrees Celsius.
    pub temperature_c: f32,
    /// Mean annual precipitation in millimeters per year.
    pub moisture_mm: f32,
    /// Continentalness factor `[0,1]`. Higher = continental interior
    /// (drier, more variable); lower = oceanic (wetter, more moderated).
    /// Mirrors `TerrainNoise::sample_continental_01` semantics so D.3 can
    /// share the same source field across both subsystems if desired.
    pub continentalness: f32,
    /// Phase 1.X-F.1.B: low-frequency Perlin noise field representing
    /// flatness propensity (range `[-1, 1]`). High erosion → flat terrain;
    /// low erosion → mountainous, per Minecraft 1.18+ canonical
    /// interpretation. Read by per-archetype `BootstrapSplineSet` in
    /// F.2-F.3 to produce per-region shape variation. Unused by D.2's
    /// `lookup_biome` and D.4's biome blending; populated unconditionally.
    pub erosion: f32,
    /// Phase 1.X-F.1.B: low-frequency Perlin noise field that feeds the
    /// PV (Peaks-and-Valleys) fold (range `[-1, 1]`). Use [`Self::pv`]
    /// to compute the canonical Minecraft 1.18+ folded value
    /// (`pv = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()`). Read by
    /// per-archetype `BootstrapSplineSet` in F.2-F.3. Unused by D.2 and
    /// D.4; populated unconditionally.
    pub weirdness: f32,
}

impl ClimateSample {
    /// Phase 1.X-F.1.B: derived Peaks-and-Valleys field via the
    /// canonical Minecraft 1.18+ formula
    /// `pv = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()`.
    ///
    /// Folded weirdness produces the characteristic 5-band character
    /// (Valleys / Low / Mid / High / Peaks). Output range `[-1, 1]` for
    /// `|weirdness| ∈ [0, 1]`.
    ///
    /// Computed at call time (not stored); see
    /// [`crate::spline_types::PvFold::from_weirdness`] for the
    /// underlying helper.
    #[inline]
    pub fn pv(&self) -> f32 {
        crate::spline_types::PvFold::from_weirdness(self.weirdness)
    }
}

/// Phase 1.6-F.4.B.3.D.1: a world archetype is a climate envelope —
/// means and variances for the three climate fields, plus a latitude
/// strength scalar controlling how steeply temperature drops toward
/// the world's edge in Z.
///
/// Archetypes do NOT assign biomes directly. Per the F.4.B.3.D
/// architecture, biomes emerge from per-vertex climate × elevation
/// lookup (D.2). Archetypes shift the *distribution* of climate values
/// across the world; specific biomes that result depend on the
/// Whittaker lookup applied to those values.
///
/// D.1 ships a single `Default` impl corresponding to Continental
/// Temperate (Veilweaver default — NC/Appalachia analog). D.5 adds the
/// other five archetypes (Equatorial Tropical, Boreal/Subarctic,
/// Mediterranean, Desert, Custom) and the editor UI surface.
///
/// Phase 1.X-F.2.C: extended with `bootstrap_splines: BootstrapSplineSet`
/// field for the regional archetype variation campaign's per-archetype
/// shape splines. **`Copy` derive removed** because `BootstrapSplineSet`
/// contains `Vec<(f32, f32)>` (in `Spline1D::control_points`), which
/// is not `Copy`. This is a documented deviation logged in
/// `REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` §10 F.2 entry; F.7 will
/// inevitably need multi-control-point splines that are intrinsically
/// non-Copy, so removing Copy now avoids a later forced refactor.
/// Existing `Clone` derive is preserved; call sites that previously
/// relied on implicit copy semantics now use explicit `.clone()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldArchetype {
    /// Mean temperature at world center, sea level, before lapse rate.
    pub temperature_mean_c: f32,
    /// Per-axis temperature noise variance (±°C). Noise field is sampled
    /// to `[-1, 1]` and multiplied by this.
    pub temperature_variance_c: f32,
    /// Temperature drop from equator (latitude=0) to pole-edge
    /// (latitude=±1) in °C. Aesthetic scaling, not physically calibrated;
    /// the world is treated as a slice of latitude rather than a globe.
    pub latitude_temperature_drop_c: f32,
    /// Mean moisture at world center, sea level. mm/year.
    pub moisture_mean_mm: f32,
    /// Per-axis moisture noise variance (±mm/yr).
    pub moisture_variance_mm: f32,
    /// Continentalness mean in `[0,1]`. Coastal (low) vs interior (high).
    pub continentalness_mean: f32,
    /// Continentalness noise variance.
    pub continentalness_variance: f32,
    /// Phase 1.X-F.2.C: per-archetype bootstrap noise parameter splines.
    /// Each archetype's catalog factory function (in
    /// `crate::world_archetypes::*`) populates this with single-control-point
    /// splines at F.4.B.3.D.5-fix baseline values. F.7 differentiates per
    /// archetype with multi-control-point shapes.
    ///
    /// `#[serde(skip, default)]` so worlds serialized before F.2.C
    /// deserialize cleanly with the baseline `BootstrapSplineSet` as
    /// fallback. Since `BootstrapSplineSet` contains a `Spline1D::Vec`,
    /// it cannot be serialized via `Copy`-derived bincode/postcard;
    /// the architectural intent is that splines live as compile-time
    /// data populated by catalog factory calls, not as serialized world
    /// state.
    #[serde(skip, default)]
    pub bootstrap_splines: crate::spline_types::BootstrapSplineSet,
}

impl Default for WorldArchetype {
    /// Continental Temperate (Veilweaver default — NC/Appalachia analog).
    /// Temperature mean 12°C with strong latitude effect; moisture mean
    /// 1100mm with moderate variance; continentalness mean 0.5 (mixed
    /// coast and interior).
    ///
    /// Phase 1.6-F.4.B.3.D.5: parameter values lifted into the canonical
    /// catalog at `crate::world_archetypes::continental_temperate()`.
    /// `default()` delegates to that function for symmetry across the
    /// six-archetype catalog. moisture_variance 600 → 400 and
    /// continentalness_variance 0.25 → 0.2 from D.1's initial values
    /// per D.5 §1.1's tuned envelope; downstream tests (D.4 blending,
    /// D.2 distribution) are robust to this small variance change.
    fn default() -> Self {
        crate::world_archetypes::continental_temperate()
    }
}

impl WorldArchetype {
    /// Validate parameter ranges. Returns `Err` with description if any
    /// parameter is out of range. Used by tests + future archetype
    /// validation in D.5.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.temperature_mean_c.is_finite() {
            return Err("temperature_mean_c must be finite");
        }
        if !(TEMPERATURE_C_MIN..=TEMPERATURE_C_MAX).contains(&self.temperature_mean_c) {
            return Err("temperature_mean_c outside [-30, +40] range");
        }
        if self.temperature_variance_c < 0.0 || self.temperature_variance_c > 30.0 {
            return Err("temperature_variance_c outside [0, 30] range");
        }
        if self.latitude_temperature_drop_c < 0.0 || self.latitude_temperature_drop_c > 50.0 {
            return Err("latitude_temperature_drop_c outside [0, 50] range");
        }
        if !(MOISTURE_MM_MIN..=MOISTURE_MM_MAX).contains(&self.moisture_mean_mm) {
            return Err("moisture_mean_mm outside [0, 4000] range");
        }
        if self.moisture_variance_mm < 0.0 || self.moisture_variance_mm > 2000.0 {
            return Err("moisture_variance_mm outside [0, 2000] range");
        }
        if !(0.0..=1.0).contains(&self.continentalness_mean) {
            return Err("continentalness_mean outside [0, 1] range");
        }
        if self.continentalness_variance < 0.0 || self.continentalness_variance > 0.5 {
            return Err("continentalness_variance outside [0, 0.5] range");
        }
        Ok(())
    }
}

/// Configuration for climate generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateConfig {
    /// Temperature noise settings
    pub temperature: ClimateLayer,
    /// Moisture noise settings
    pub moisture: ClimateLayer,
    /// Phase 1.6-F.4.B.3.D.1: continentalness noise settings. Low-frequency
    /// noise field producing the continentalness `[0,1]` value when
    /// combined with archetype mean/variance. Wavelength chosen to mirror
    /// `TerrainNoise::sample_continental_01` (default scale 0.0003 ≈
    /// 3300 WU wavelength, ~3 periods across radius-10 world).
    #[serde(default = "default_continentalness_layer")]
    pub continentalness: ClimateLayer,
    /// Height influence on temperature (degrees per meter).
    /// **Legacy**: kept for backward-compat with `sample_temperature` /
    /// `sample_moisture` / `sample_climate` (returning `[0,1]` values).
    /// New `sample()` API uses `ATMOSPHERIC_LAPSE_RATE_C_PER_M` constant.
    pub temperature_height_gradient: f32,
    /// Latitude influence on temperature.
    /// **Legacy**: kept for backward-compat with `sample_temperature`.
    /// New `sample()` API uses `WorldArchetype::latitude_temperature_drop_c`.
    pub temperature_latitude_gradient: f32,
    /// Distance from water influence on moisture.
    /// Used by both legacy `sample_moisture` and new `sample()` (the
    /// water-distance modulator's falloff rate; the approximation method
    /// itself differs — legacy uses noise-threshold, new uses
    /// distance-from-world-edge).
    pub moisture_distance_falloff: f32,
    /// Phase 1.6-F.4.B.3.D.1: world archetype. Defines climate envelope
    /// means and variances. `Default` → Continental Temperate.
    #[serde(default)]
    pub archetype: WorldArchetype,
    /// Phase 1.6-F.4.B.3.D.1: world half-extent for latitude normalization
    /// (in world units). Default `TARGET_B_LATITUDE_HALF_EXTENT_WU`.
    /// Override for non-Target-B world scales (tests, future Target C).
    #[serde(default = "default_world_latitude_half_extent")]
    pub world_latitude_half_extent_wu: f32,
}

fn default_continentalness_layer() -> ClimateLayer {
    // Mirrors TerrainNoise's continental_scale = 0.0003 (≈3300 WU
    // wavelength) with single octave (continental modulation is intended
    // to be smooth large-scale; multi-octave would inject mid-frequency
    // detail that overlaps the per-archetype variance role).
    ClimateLayer {
        scale: 0.0003,
        octaves: 1,
        persistence: 0.5,
        lacunarity: 2.0,
        amplitude: 1.0,
        offset: 0.0,
    }
}

fn default_world_latitude_half_extent() -> f32 {
    TARGET_B_LATITUDE_HALF_EXTENT_WU
}

impl Default for ClimateConfig {
    fn default() -> Self {
        Self {
            temperature: ClimateLayer {
                scale: 0.001,
                octaves: 3,
                persistence: 0.5,
                lacunarity: 2.0,
                amplitude: 1.0,
                offset: 0.5,
            },
            moisture: ClimateLayer {
                scale: 0.0015,
                octaves: 4,
                persistence: 0.6,
                lacunarity: 2.1,
                amplitude: 1.0,
                offset: 0.5,
            },
            continentalness: default_continentalness_layer(),
            temperature_height_gradient: -0.0065, // Standard atmospheric lapse rate
            temperature_latitude_gradient: 0.8,   // Stronger temperature variation by latitude
            moisture_distance_falloff: 0.001,     // Moisture decreases inland
            archetype: WorldArchetype::default(),
            world_latitude_half_extent_wu: default_world_latitude_half_extent(),
        }
    }
}

/// Configuration for a single climate layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateLayer {
    /// Noise scale (frequency)
    pub scale: f64,
    /// Number of noise octaves
    pub octaves: usize,
    /// Persistence (amplitude falloff between octaves)
    pub persistence: f64,
    /// Lacunarity (frequency multiplier between octaves)
    pub lacunarity: f64,
    /// Overall amplitude
    pub amplitude: f32,
    /// Base offset value
    pub offset: f32,
}

/// Climate map generator that provides temperature and moisture data
#[derive(Debug)]
pub struct ClimateMap {
    temperature_noise: Perlin,
    moisture_noise: Perlin,
    /// Phase 1.6-F.4.B.3.D.1: low-frequency continentalness noise.
    /// Sampled by the new `sample()` API. Seed is `seed + 2000` to
    /// decorrelate from temperature (`seed`) and moisture (`seed + 1000`).
    continentalness_noise: Perlin,
    /// Phase 1.X-F.1.B: low-frequency erosion noise. Seed is `seed + 3000`
    /// to decorrelate from existing climate fields. Scale 0.0008
    /// (wavelength ~1250 WU at Target B). Output `[-1, 1]` represents
    /// flatness propensity per Minecraft 1.18+ canonical interpretation.
    erosion_noise: Perlin,
    /// Phase 1.X-F.1.B: low-frequency weirdness noise. Seed is `seed + 4000`
    /// to decorrelate from existing climate fields. Scale 0.0006
    /// (wavelength ~1670 WU). Output `[-1, 1]` feeds the PV fold via
    /// [`ClimateSample::pv`].
    weirdness_noise: Perlin,
    config: ClimateConfig,
}

impl ClimateMap {
    /// Create a new climate map generator
    pub fn new(config: &ClimateConfig, seed: u64) -> Self {
        Self {
            temperature_noise: Perlin::new(seed as u32),
            moisture_noise: Perlin::new((seed + 1000) as u32),
            continentalness_noise: Perlin::new((seed + 2000) as u32),
            // Phase 1.X-F.1.B: erosion + weirdness noise for the
            // regional archetype variation campaign's shape splines.
            // Offsets +3000 / +4000 verified clear of all other terrain
            // crate seed offsets in F.1 prompt §1.1 verification step.
            erosion_noise: Perlin::new((seed + 3000) as u32),
            weirdness_noise: Perlin::new((seed + 4000) as u32),
            config: config.clone(),
        }
    }

    /// Phase 1.6-F.4.B.3.D.1: real-units climate sample at a world position.
    ///
    /// Returns `(temperature_c, moisture_mm, continentalness)` driven by the
    /// configured `WorldArchetype` envelope plus three independent modulators:
    ///
    /// 1. **Latitude modulator** — temperature drops toward world edges in Z
    ///    by `archetype.latitude_temperature_drop_c` × `|world_z / world_latitude_half_extent_wu|`.
    /// 2. **Elevation lapse rate** — temperature drops by 6.5°C per 1000m
    ///    elevation (`ATMOSPHERIC_LAPSE_RATE_C_PER_M`).
    /// 3. **Water-distance modulator** — moisture decreases with distance
    ///    from world edge (continental interior is drier). Falloff rate
    ///    controlled by `config.moisture_distance_falloff`.
    ///
    /// Output values are clamped to `[TEMPERATURE_C_MIN, TEMPERATURE_C_MAX]`,
    /// `[MOISTURE_MM_MIN, MOISTURE_MM_MAX]`, and `[0, 1]` respectively.
    ///
    /// New per-vertex API for D.2/D.3 Whittaker biome lookup. Existing
    /// consumers (`biome_detector`, `biome_transition`, renderer overlay)
    /// still use the legacy `[0,1]`-bounded `sample_climate`/`sample_temperature`/
    /// `sample_moisture` methods until D.3 migrates them.
    pub fn sample(&self, world_x: f64, world_z: f64, elevation: f32) -> ClimateSample {
        let arch = &self.config.archetype;

        // === Temperature ===
        // Base: archetype mean + low-frequency noise variance.
        let temp_noise_raw = self.sample_noise_signed(
            &self.temperature_noise,
            &self.config.temperature,
            world_x,
            world_z,
        );
        let mut temperature_c =
            arch.temperature_mean_c + temp_noise_raw * arch.temperature_variance_c;

        // Latitude modulator: temperature decreases toward poles.
        // |z / half_extent| ∈ [0, 1+] (clamped). At equator (lat=0) no
        // adjustment; at pole-edge (|lat|=1) full archetype drop applied.
        let latitude_norm = self.latitude_normalized(world_z);
        temperature_c -= latitude_norm.abs() * arch.latitude_temperature_drop_c;

        // Elevation lapse rate: standard atmospheric -6.5°C per 1000m.
        // Below sea level (negative elevation) gets a small temperature
        // bump but the clamp at `TEMPERATURE_C_MAX` prevents runaway.
        temperature_c += elevation * ATMOSPHERIC_LAPSE_RATE_C_PER_M;

        let temperature_c = temperature_c.clamp(TEMPERATURE_C_MIN, TEMPERATURE_C_MAX);

        // === Moisture ===
        // Base: archetype mean + low-frequency noise variance.
        let moist_noise_raw = self.sample_noise_signed(
            &self.moisture_noise,
            &self.config.moisture,
            world_x,
            world_z,
        );
        let mut moisture_mm =
            arch.moisture_mean_mm + moist_noise_raw * arch.moisture_variance_mm;

        // Water-distance modulator: distance from world edge proxies for
        // continental interior. World edge → high moisture (coast); world
        // center → low moisture (interior). `coast_distance` is the
        // shortest distance to either x or z world edge.
        let half_ext = self.config.world_latitude_half_extent_wu as f64;
        let coast_distance =
            (half_ext - world_x.abs()).min(half_ext - world_z.abs()).max(0.0) as f32;
        let coast_factor =
            (-coast_distance * self.config.moisture_distance_falloff).exp();
        // Mix: 70% noise-driven, 30% coast-driven. Coastal regions get a
        // moisture boost up to 30% of the archetype mean.
        moisture_mm = moisture_mm * 0.7 + (arch.moisture_mean_mm * coast_factor) * 0.3;

        let moisture_mm = moisture_mm.clamp(MOISTURE_MM_MIN, MOISTURE_MM_MAX);

        // === Continentalness ===
        // Single-octave low-frequency noise around archetype mean.
        let cont_noise_raw = self.sample_noise_signed(
            &self.continentalness_noise,
            &self.config.continentalness,
            world_x,
            world_z,
        );
        let continentalness =
            (arch.continentalness_mean + cont_noise_raw * arch.continentalness_variance)
                .clamp(0.0, 1.0);

        // === Erosion (Phase 1.X-F.1.B) ===
        // Low-frequency Perlin field at scale 0.0008 (~1250 WU wavelength
        // at Target B). Output [-1, 1] represents flatness propensity per
        // Minecraft 1.18+ canonical interpretation. Defensive clamp
        // against f32 boundary artifacts that occasionally produce
        // ±1.0001 at integer lattice positions; Perlin output is
        // mathematically bounded.
        let erosion_raw =
            self.erosion_noise.get([world_x * 0.0008, world_z * 0.0008]) as f32;
        let erosion = erosion_raw.clamp(-1.0, 1.0);

        // === Weirdness (Phase 1.X-F.1.B) ===
        // Low-frequency Perlin field at scale 0.0006 (~1670 WU wavelength
        // at Target B). Output [-1, 1] feeds the PV (Peaks-and-Valleys)
        // fold via ClimateSample::pv. Same clamp rationale as erosion.
        let weirdness_raw =
            self.weirdness_noise.get([world_x * 0.0006, world_z * 0.0006]) as f32;
        let weirdness = weirdness_raw.clamp(-1.0, 1.0);

        ClimateSample {
            temperature_c,
            moisture_mm,
            continentalness,
            erosion,
            weirdness,
        }
    }

    /// Phase 1.6-F.4.B.3.D.1: latitude normalized to `[-1, 1]+` based on
    /// world Z. Values beyond ±1 are possible if the world extends past
    /// the configured half-extent (radius >10 chunks at Target B).
    /// Exposed `pub(crate)` for D.1 unit tests.
    #[inline]
    pub(crate) fn latitude_normalized(&self, world_z: f64) -> f32 {
        if self.config.world_latitude_half_extent_wu <= 0.0 {
            0.0
        } else {
            (world_z as f32) / self.config.world_latitude_half_extent_wu
        }
    }

    /// Phase 1.6-F.4.B.3.D.1: sample fBm noise as a signed value
    /// approximately in `[-1, 1]`. Used by `sample()` so noise output can
    /// be multiplied by archetype variance without the `+ offset` bias
    /// the legacy `sample_noise_fbm` adds for `[0,1]`-style consumers.
    #[inline]
    fn sample_noise_signed(
        &self,
        noise: &Perlin,
        layer: &ClimateLayer,
        x: f64,
        z: f64,
    ) -> f32 {
        let mut value = 0.0f32;
        let mut amplitude = layer.amplitude;
        let mut frequency = layer.scale;
        for _ in 0..layer.octaves {
            value += noise.get([x * frequency, 0.0, z * frequency]) as f32 * amplitude;
            amplitude *= layer.persistence as f32;
            frequency *= layer.lacunarity;
        }
        value.clamp(-1.0, 1.0)
    }

    /// Sample temperature at a world position
    pub fn sample_temperature(&self, x: f64, z: f64, height: f32) -> f32 {
        // Base temperature from noise
        let mut temperature =
            self.sample_noise_fbm(&self.temperature_noise, &self.config.temperature, x, z);

        // Apply height gradient (cooler at higher elevations)
        temperature += height * self.config.temperature_height_gradient;

        // Apply latitude gradient (cooler towards poles)
        let latitude_factor = (z * 0.00001).sin(); // Very rough latitude simulation
        temperature += latitude_factor as f32 * self.config.temperature_latitude_gradient;

        // Clamp to reasonable range
        temperature.clamp(0.0, 1.0)
    }

    /// Sample moisture at a world position
    pub fn sample_moisture(&self, x: f64, z: f64, height: f32) -> f32 {
        // Base moisture from noise
        let mut moisture = self.sample_noise_fbm(&self.moisture_noise, &self.config.moisture, x, z);

        // Reduce moisture at higher elevations (rain shadow effect)
        let height_factor = (height * 0.01).clamp(0.0, 1.0);
        moisture *= 1.0 - height_factor * 0.3;

        // Distance from water effect (simplified - in real implementation would use actual water bodies)
        let water_distance = self.estimate_water_distance(x, z);
        let water_factor = (-water_distance * self.config.moisture_distance_falloff).exp();
        moisture = moisture * 0.7 + water_factor * 0.3;

        // Clamp to valid range
        moisture.clamp(0.0, 1.0)
    }

    /// Sample both temperature and moisture at a world position
    pub fn sample_climate(&self, x: f64, z: f64, height: f32) -> (f32, f32) {
        let temperature = self.sample_temperature(x, z, height);
        let moisture = self.sample_moisture(x, z, height);
        (temperature, moisture)
    }

    /// Sample climate data for an entire chunk
    pub fn sample_chunk(
        &self,
        chunk_id: ChunkId,
        chunk_size: f32,
        resolution: u32,
    ) -> anyhow::Result<Vec<(f32, f32)>> {
        let world_origin = chunk_id.to_world_pos(chunk_size);
        let step = chunk_size / (resolution - 1) as f32;
        let mut climate_data = Vec::with_capacity((resolution * resolution) as usize);

        for z in 0..resolution {
            for x in 0..resolution {
                let world_x = world_origin.x + x as f32 * step;
                let world_z = world_origin.z + z as f32 * step;

                // We need height data to calculate climate properly
                // For now, use a simple height estimation based on position
                let estimated_height = self.estimate_height(world_x as f64, world_z as f64);

                let climate = self.sample_climate(world_x as f64, world_z as f64, estimated_height);
                climate_data.push(climate);
            }
        }

        Ok(climate_data)
    }

    /// Sample fractal Brownian motion noise
    fn sample_noise_fbm(&self, noise: &Perlin, layer: &ClimateLayer, x: f64, z: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = layer.amplitude;
        let mut frequency = layer.scale;

        for _ in 0..layer.octaves {
            value += noise.get([x * frequency, 0.0, z * frequency]) as f32 * amplitude;
            amplitude *= layer.persistence as f32;
            frequency *= layer.lacunarity;
        }

        value + layer.offset
    }

    /// Estimate height at a position (temporary until we have proper integration)
    pub fn estimate_height(&self, x: f64, z: f64) -> f32 {
        // Simple height estimation using noise — amplitude must roughly
        // match the maximum terrain amplitude used by noise presets (e.g.
        // mountain base_amplitude 120 + mountains_amplitude 150 ≈ ~270).
        let height_noise = self.sample_noise_fbm(
            &self.temperature_noise, // Reuse temperature noise for height
            &ClimateLayer {
                scale: 0.002,
                octaves: 4,
                persistence: 0.5,
                lacunarity: 2.0,
                amplitude: 150.0,
                offset: 10.0,
            },
            x,
            z,
        );
        height_noise.max(0.0)
    }

    /// Estimate distance to nearest water body (simplified)
    fn estimate_water_distance(&self, x: f64, z: f64) -> f32 {
        // Simplified water distance using noise to create "rivers" and "lakes"
        let water_noise = self.sample_noise_fbm(
            &self.moisture_noise,
            &ClimateLayer {
                scale: 0.003,
                octaves: 2,
                persistence: 0.4,
                lacunarity: 2.5,
                amplitude: 1.0,
                offset: 0.0,
            },
            x,
            z,
        );

        // If noise is below threshold, we're "near water"
        if water_noise.abs() < 0.1 {
            0.0 // At water
        } else {
            (water_noise.abs() - 0.1) * 1000.0 // Distance in arbitrary units
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &ClimateConfig {
        &self.config
    }
}

/// Utility functions for climate analysis
pub mod utils {
    use super::*;
    use crate::BiomeType;

    /// Classify biome based on temperature and moisture (Whittaker biome classification)
    pub fn classify_whittaker_biome(temperature: f32, moisture: f32) -> BiomeType {
        match (temperature, moisture) {
            (t, _m) if t < 0.2 => BiomeType::Tundra,
            (t, m) if t < 0.4 && m < 0.3 => BiomeType::Tundra,
            (t, m) if t < 0.6 && m < 0.2 => BiomeType::Desert,
            (t, m) if t > 0.7 && m < 0.4 => BiomeType::Desert,
            (_t, m) if m > 0.8 => BiomeType::Swamp,
            (t, m) if t > 0.6 && m > 0.6 => BiomeType::Forest,
            (t, m) if t > 0.4 && m > 0.4 => BiomeType::Forest,
            _ => BiomeType::Grassland,
        }
    }

    /// Generate a climate preview for visualization
    pub fn generate_climate_preview(
        climate: &ClimateMap,
        size: u32,
        scale: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut temperatures = Vec::with_capacity((size * size) as usize);
        let mut moistures = Vec::with_capacity((size * size) as usize);
        let step = scale / size as f32;

        for z in 0..size {
            for x in 0..size {
                let world_x = x as f32 * step;
                let world_z = z as f32 * step;
                let height = climate.estimate_height(world_x as f64, world_z as f64);

                let (temperature, moisture) =
                    climate.sample_climate(world_x as f64, world_z as f64, height);

                temperatures.push(temperature);
                moistures.push(moisture);
            }
        }

        (temperatures, moistures)
    }

    /// Create a biome classification map
    pub fn generate_biome_classification_map(
        climate: &ClimateMap,
        size: u32,
        scale: f32,
    ) -> Vec<BiomeType> {
        let mut biomes = Vec::with_capacity((size * size) as usize);
        let step = scale / size as f32;

        for z in 0..size {
            for x in 0..size {
                let world_x = x as f32 * step;
                let world_z = z as f32 * step;
                let height = climate.estimate_height(world_x as f64, world_z as f64);

                let (temperature, moisture) =
                    climate.sample_climate(world_x as f64, world_z as f64, height);

                let biome = classify_whittaker_biome(temperature, moisture);
                biomes.push(biome);
            }
        }

        biomes
    }

    /// Calculate climate statistics for a region
    pub fn calculate_climate_stats(
        climate: &ClimateMap,
        min_x: f64,
        max_x: f64,
        min_z: f64,
        max_z: f64,
        samples: u32,
    ) -> ClimateStats {
        let mut temperatures = Vec::new();
        let mut moistures = Vec::new();

        let step_x = (max_x - min_x) / samples as f64;
        let step_z = (max_z - min_z) / samples as f64;

        for i in 0..samples {
            for j in 0..samples {
                let x = min_x + i as f64 * step_x;
                let z = min_z + j as f64 * step_z;
                let height = climate.estimate_height(x, z);

                let (temperature, moisture) = climate.sample_climate(x, z, height);
                temperatures.push(temperature);
                moistures.push(moisture);
            }
        }

        ClimateStats {
            temperature_min: temperatures.iter().copied().fold(f32::INFINITY, f32::min),
            temperature_max: temperatures
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max),
            temperature_avg: temperatures.iter().sum::<f32>() / temperatures.len() as f32,
            moisture_min: moistures.iter().copied().fold(f32::INFINITY, f32::min),
            moisture_max: moistures.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            moisture_avg: moistures.iter().sum::<f32>() / moistures.len() as f32,
        }
    }

    /// Climate statistics for a region
    #[derive(Debug, Clone)]
    pub struct ClimateStats {
        pub temperature_min: f32,
        pub temperature_max: f32,
        pub temperature_avg: f32,
        pub moisture_min: f32,
        pub moisture_max: f32,
        pub moisture_avg: f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BiomeType;

    #[test]
    fn test_climate_config_default() {
        let config = ClimateConfig::default();
        assert!(config.temperature.scale > 0.0);
        assert!(config.moisture.scale > 0.0);
    }

    #[test]
    fn test_climate_map_creation() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let (temperature, moisture) = climate.sample_climate(100.0, 100.0, 10.0);
        assert!((0.0..=1.0).contains(&temperature));
        assert!((0.0..=1.0).contains(&moisture));
    }

    #[test]
    fn test_height_gradient() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let temp_low = climate.sample_temperature(100.0, 100.0, 0.0);
        let temp_high = climate.sample_temperature(100.0, 100.0, 100.0);

        // Higher elevation should be cooler
        assert!(temp_high < temp_low);
    }

    #[test]
    fn test_chunk_sampling() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let chunk_id = ChunkId::new(0, 0);
        let climate_data = climate.sample_chunk(chunk_id, 256.0, 32).unwrap();

        assert_eq!(climate_data.len(), 32 * 32);
        for (temp, moisture) in climate_data {
            assert!((0.0..=1.0).contains(&temp));
            assert!((0.0..=1.0).contains(&moisture));
        }
    }

    #[test]
    fn test_deterministic_climate() {
        let config = ClimateConfig::default();
        let climate1 = ClimateMap::new(&config, 12345);
        let climate2 = ClimateMap::new(&config, 12345);

        let (temp1, moisture1) = climate1.sample_climate(100.0, 100.0, 10.0);
        let (temp2, moisture2) = climate2.sample_climate(100.0, 100.0, 10.0);

        assert_eq!(temp1, temp2);
        assert_eq!(moisture1, moisture2);
    }

    #[test]
    fn test_whittaker_classification() {
        assert_eq!(utils::classify_whittaker_biome(0.1, 0.5), BiomeType::Tundra);
        assert_eq!(utils::classify_whittaker_biome(0.8, 0.1), BiomeType::Desert);
        assert_eq!(utils::classify_whittaker_biome(0.7, 0.9), BiomeType::Swamp);
        assert_eq!(utils::classify_whittaker_biome(0.7, 0.7), BiomeType::Forest);
        assert_eq!(utils::classify_whittaker_biome(0.5, 0.5), BiomeType::Forest);
    }

    #[test]
    fn test_climate_preview() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let (temperatures, moistures) = utils::generate_climate_preview(&climate, 16, 256.0);

        assert_eq!(temperatures.len(), 16 * 16);
        assert_eq!(moistures.len(), 16 * 16);
    }

    #[test]
    fn test_biome_classification_map() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let biomes = utils::generate_biome_classification_map(&climate, 16, 256.0);

        assert_eq!(biomes.len(), 16 * 16);
        assert!(biomes.iter().all(|b| BiomeType::all().contains(b)));
    }

    // ============================================================
    // Phase 1.6-F.4.B.3.D.1: real-units climate field tests.
    // ============================================================

    #[test]
    fn phase_1_6_f4_b_3_d_1_default_archetype_validates() {
        let arch = WorldArchetype::default();
        arch.validate()
            .expect("default WorldArchetype (Continental Temperate) must validate");
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_validate_rejects_out_of_range() {
        let bad = WorldArchetype {
            temperature_mean_c: 100.0, // > +40°C max
            ..WorldArchetype::default()
        };
        assert!(bad.validate().is_err());

        let bad = WorldArchetype {
            moisture_mean_mm: -1.0,
            ..WorldArchetype::default()
        };
        assert!(bad.validate().is_err());

        let bad = WorldArchetype {
            continentalness_mean: 1.5,
            ..WorldArchetype::default()
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_sample_returns_real_units_within_bounds() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        // Sample a 16×16 grid spanning the world. All values must be
        // in their documented ranges.
        let half = TARGET_B_LATITUDE_HALF_EXTENT_WU as f64;
        let step = (2.0 * half) / 15.0;
        for i in 0..16 {
            for j in 0..16 {
                let x = -half + (i as f64) * step;
                let z = -half + (j as f64) * step;
                let elevation = ((i + j) * 50) as f32; // 0..1500 m sweep

                let s = climate.sample(x, z, elevation);
                assert!(
                    s.temperature_c.is_finite(),
                    "temperature_c must be finite at ({x}, {z}, {elevation})"
                );
                assert!(
                    (TEMPERATURE_C_MIN..=TEMPERATURE_C_MAX).contains(&s.temperature_c),
                    "temperature_c out of bounds: {} at ({x}, {z}, {elevation})",
                    s.temperature_c
                );
                assert!(
                    s.moisture_mm.is_finite(),
                    "moisture_mm must be finite at ({x}, {z}, {elevation})"
                );
                assert!(
                    (MOISTURE_MM_MIN..=MOISTURE_MM_MAX).contains(&s.moisture_mm),
                    "moisture_mm out of bounds: {} at ({x}, {z}, {elevation})",
                    s.moisture_mm
                );
                assert!(
                    s.continentalness.is_finite(),
                    "continentalness must be finite"
                );
                assert!(
                    (0.0..=1.0).contains(&s.continentalness),
                    "continentalness out of bounds: {}",
                    s.continentalness
                );
            }
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_sample_is_deterministic() {
        let config = ClimateConfig::default();
        let a = ClimateMap::new(&config, 12345);
        let b = ClimateMap::new(&config, 12345);

        let s1 = a.sample(123.0, 456.0, 100.0);
        let s2 = b.sample(123.0, 456.0, 100.0);

        assert_eq!(s1, s2, "sample must be deterministic for same seed + inputs");
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_latitude_modulator_drops_temperature_at_poles() {
        // Use an archetype with strong latitude effect and disabled noise
        // variance to isolate the latitude modulator.
        let arch = WorldArchetype {
            temperature_mean_c: 20.0,
            temperature_variance_c: 0.0, // disable temperature noise variance
            latitude_temperature_drop_c: 15.0,
            ..WorldArchetype::default()
        };
        let mut config = ClimateConfig::default();
        config.archetype = arch;
        config.moisture.amplitude = 0.0; // also stabilize moisture noise
        let climate = ClimateMap::new(&config, 7);

        let half = TARGET_B_LATITUDE_HALF_EXTENT_WU as f64;
        // At equator (z=0), temperature should be near mean (with noise + lapse).
        let s_eq = climate.sample(0.0, 0.0, 0.0);
        // At pole-edge (z=+half), temperature should be lower by
        // latitude_temperature_drop_c (15°C in this test).
        let s_pole = climate.sample(0.0, half, 0.0);
        let drop = s_eq.temperature_c - s_pole.temperature_c;
        assert!(
            drop > 12.0 && drop < 18.0,
            "expected ~15°C latitude drop with full latitude_temperature_drop=15.0; got {drop}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_elevation_lapse_rate_drops_temperature_at_altitude() {
        let mut config = ClimateConfig::default();
        config.archetype.temperature_variance_c = 0.0;
        config.moisture.amplitude = 0.0;
        let climate = ClimateMap::new(&config, 11);

        let s_low = climate.sample(0.0, 0.0, 0.0);
        let s_high = climate.sample(0.0, 0.0, 1000.0);
        let drop = s_low.temperature_c - s_high.temperature_c;
        // 6.5°C drop per 1000m elevation expected.
        assert!(
            drop > 6.0 && drop < 7.0,
            "expected ~6.5°C lapse-rate drop over 1000m; got {drop}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_water_distance_modulator_dries_continental_interior() {
        let arch = WorldArchetype {
            moisture_mean_mm: 1500.0,
            moisture_variance_mm: 0.0,
            ..WorldArchetype::default()
        };
        let mut config = ClimateConfig::default();
        config.archetype = arch;
        config.temperature.amplitude = 0.0;
        // Larger falloff so the effect is detectable across the world half-extent.
        config.moisture_distance_falloff = 0.001;
        let climate = ClimateMap::new(&config, 13);

        // World center (x=0, z=0) is maximum coast_distance → highest interior dryness.
        let s_center = climate.sample(0.0, 0.0, 0.0);
        // Near a world edge (x near +half_extent) is coastal → wetter than interior.
        let half = TARGET_B_LATITUDE_HALF_EXTENT_WU as f64;
        let s_coast = climate.sample(half - 100.0, 0.0, 0.0);
        assert!(
            s_coast.moisture_mm > s_center.moisture_mm,
            "coastal moisture ({}) should exceed interior moisture ({})",
            s_coast.moisture_mm,
            s_center.moisture_mm
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_sample_grid_distribution_matches_archetype_mean() {
        // Sample a 16×16 grid (256 samples) and verify the grand mean
        // approximately matches the archetype mean. Tolerance is generous
        // because lapse rate, latitude, and water-distance modulators all
        // shift the sampled mean from the raw archetype mean.
        let config = ClimateConfig::default();
        // Phase 1.X-F.2.C: WorldArchetype no longer Copy (BootstrapSplineSet
        // contains Vec); clone to avoid partial move.
        let arch = config.archetype.clone();
        let climate = ClimateMap::new(&config, 12345);

        let half = TARGET_B_LATITUDE_HALF_EXTENT_WU as f64;
        let step = (2.0 * half) / 15.0;
        let mut temps = Vec::with_capacity(256);
        let mut moists = Vec::with_capacity(256);
        let mut conts = Vec::with_capacity(256);
        for i in 0..16 {
            for j in 0..16 {
                let x = -half + (i as f64) * step;
                let z = -half + (j as f64) * step;
                // Sea-level samples to avoid lapse rate biasing toward cold.
                let s = climate.sample(x, z, 0.0);
                temps.push(s.temperature_c);
                moists.push(s.moisture_mm);
                conts.push(s.continentalness);
            }
        }
        let mean_temp = temps.iter().sum::<f32>() / temps.len() as f32;
        let mean_moist = moists.iter().sum::<f32>() / moists.len() as f32;
        let mean_cont = conts.iter().sum::<f32>() / conts.len() as f32;

        // Temperature: archetype mean - half of latitude drop (avg |lat|≈0.5).
        let expected_temp = arch.temperature_mean_c - 0.5 * arch.latitude_temperature_drop_c;
        assert!(
            (mean_temp - expected_temp).abs() < 5.0,
            "sample mean temp {mean_temp} should be near {expected_temp} ± 5°C"
        );

        // Moisture: roughly archetype mean (water-distance modulator boosts
        // coastal samples; sample variance from noise is centered on 0).
        // Tolerance: ±25% of mean.
        let moisture_tolerance = arch.moisture_mean_mm * 0.25;
        assert!(
            (mean_moist - arch.moisture_mean_mm).abs() < moisture_tolerance,
            "sample mean moisture {mean_moist} should be near {} ± {moisture_tolerance}",
            arch.moisture_mean_mm
        );

        // Continentalness: should be near archetype mean ± noise.
        assert!(
            (mean_cont - arch.continentalness_mean).abs() < 0.15,
            "sample mean continentalness {mean_cont} should be near {} ± 0.15",
            arch.continentalness_mean
        );

        // No NaN, no all-identical values.
        assert!(temps.iter().all(|t| t.is_finite()));
        let temp_min = temps.iter().copied().fold(f32::INFINITY, f32::min);
        let temp_max = temps.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            (temp_max - temp_min) > 1.0,
            "temperatures must vary (got range {})",
            temp_max - temp_min
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_continentalness_field_varies() {
        // The new continentalness field should produce different values
        // at different positions (smoke test that the noise sampling works).
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);
        let s1 = climate.sample(0.0, 0.0, 0.0);
        let s2 = climate.sample(3000.0, 3000.0, 0.0);
        let s3 = climate.sample(-3000.0, -3000.0, 0.0);
        assert!(
            (s1.continentalness - s2.continentalness).abs() > 1e-4
                || (s1.continentalness - s3.continentalness).abs() > 1e-4,
            "continentalness must differ across positions"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_1_legacy_sample_climate_still_works() {
        // Backward-compat invariant: existing biome_detector / biome_transition
        // / renderer overlay consumers calling `sample_climate(x, z, height)`
        // must continue receiving `[0, 1]`-bounded values. D.1 must not
        // disturb this until D.3 migrates the consumers.
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);
        let (t, m) = climate.sample_climate(100.0, 200.0, 50.0);
        assert!((0.0..=1.0).contains(&t), "legacy temperature out of [0,1]");
        assert!((0.0..=1.0).contains(&m), "legacy moisture out of [0,1]");
    }
}
