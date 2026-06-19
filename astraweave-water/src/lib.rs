//! # astraweave-water — gameplay water-truth facade
//!
//! The single logical owner of **gameplay water truth**: given a world
//! position, *is there water here, how high is its surface, and how dense is
//! it?* This is the §7.7 "no second implementation" mandate made into
//! architecture — physics (and later AI/gameplay) ask their water questions
//! through [`WaterQuery`] instead of each carrying a private notion of "the
//! water".
//!
//! ## Determinism contract (engine fluids carve-out, gate Q1)
//!
//! This layer is **gameplay truth** and is therefore **CPU-resident and
//! deterministic by construction**: no GPU reads, no float-nondeterministic
//! reductions, no hashing-iteration-order dependence. Two freshly-constructed
//! backends holding the same volumes return bit-identical [`WaterSample`]s for
//! the same query, and a sample depends only on the *set* of registered
//! volumes and the query point — never on query order. GPU particle fluid
//! state (`astraweave-fluids`) is explicitly **excluded** from this layer:
//! it is presentation-only and non-deterministic. A future grid/voxel backend
//! ([`WaterQuery`] is a trait precisely so F.3 can add one) must preserve this
//! contract.
//!
//! ## Scope (F.2)
//!
//! The API surface is defined by what its **first real consumer**
//! (`astraweave-physics` buoyancy) actually calls — nothing speculative. The
//! only question physics asks of the water is "[`sample`](WaterQuery::sample)
//! this point", so that is the only method on the trait. The only backend is
//! [`AnalyticWater`] (an infinite plane for the retired scalar-`water_level`
//! compatibility, plus bounded AABB volumes for `add_water_aabb`). Flow,
//! drag, temperature, and submersion-fraction are **not** exposed because no
//! wired consumer reads them yet — they arrive together with the consumer
//! that needs them (swim/flow forces: F.2-followup/F.3).

use glam::Vec3;

/// What a consumer learns about the water at a point.
///
/// Returned by [`WaterQuery::sample`]. Carries exactly the two quantities the
/// wired physics buoyancy path reads — `surface_height` (to decide whether a
/// body is submerged: `body_y < surface_height`) and `density` (for the
/// Archimedes force `volume * density * g`). It deliberately does **not**
/// carry flow/drag/temperature: physics computes drag from a per-body
/// coefficient and never queries the water for it, so exposing those here
/// would be the dormant-speculative-API anti-pattern F.2 exists to avoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterSample {
    /// World-space Y of the water surface above this point's XZ column.
    pub surface_height: f32,
    /// Fluid density at this point (kg/m³; fresh water ≈ 1000).
    pub density: f32,
}

/// The read-side abstraction physics depends on.
///
/// One method, because the sole F.2 consumer asks exactly one question. F.3's
/// voxel backend implements the same trait; the determinism contract in the
/// crate docs is part of the contract.
pub trait WaterQuery {
    /// Water properties at `point`, or `None` if no water covers it.
    ///
    /// "Covers" is backend-defined: for [`AnalyticWater`], a point is covered
    /// by the infinite plane (if set) regardless of height, and by an AABB
    /// volume when it lies within that box. Returning `Some` does **not** mean
    /// the point is *below* the surface — the caller compares `point.y` to
    /// [`WaterSample::surface_height`] itself (preserving the wired buoyancy
    /// semantics). When several volumes cover the point, the result reflects
    /// the **topmost** surface (a body floats on the highest water), ties
    /// broken by registration order.
    fn sample(&self, point: Vec3) -> Option<WaterSample>;
}

/// An infinite flat water plane — the analytic representation of the retired
/// scalar `water_level`/`fluid_density` pair. Infinite in XZ and downward, so
/// it reproduces the old "any body below `water_level` floats" behavior
/// exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Plane {
    surface_height: f32,
    density: f32,
}

/// A bounded axis-aligned water volume (a pool, a tank): `add_water_aabb`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Aabb {
    min: Vec3,
    max: Vec3,
    density: f32,
    /// Per-volume linear drag coefficient. **Authoring data only in F.2**: the
    /// wired buoyancy path applies drag from a *per-body* coefficient and does
    /// not read this. Stored honestly (so `add_water_aabb`'s parameter is not
    /// silently discarded) and available to the F.3+ consumer that introduces
    /// per-volume drag. Not read by [`AnalyticWater::sample`].
    linear_drag: f32,
}

impl Aabb {
    /// Surface (top face) Y.
    #[inline]
    fn surface_height(&self) -> f32 {
        self.max.y
    }

    /// Full-AABB containment: a body is "in" this water body only within the
    /// box (laterally *and* vertically). A body below `min.y` is beneath the
    /// pool, not in it — so it gets no buoyancy, unlike the infinite plane.
    #[inline]
    fn contains(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }
}

/// The F.2 analytic [`WaterQuery`] backend: an optional infinite plane plus
/// any number of bounded AABB volumes. Deterministic, CPU-only.
///
/// This is the **single owner** of analytic water state. The scalar
/// `water_level`/`fluid_density` that physics still exposes for backward
/// compatibility are not a parallel store — physics write-through-syncs them
/// into [`set_plane`](AnalyticWater::set_plane) and then queries *only*
/// through this type, so there is exactly one sampling implementation.
#[derive(Clone, Debug, Default)]
pub struct AnalyticWater {
    plane: Option<Plane>,
    aabbs: Vec<Aabb>,
}

impl AnalyticWater {
    /// Empty — no water anywhere.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set (or replace) the infinite plane. A non-finite `surface_height`
    /// clears it — this is how the retired `water_level = NEG_INFINITY`
    /// "no water" sentinel maps onto the facade.
    pub fn set_plane(&mut self, surface_height: f32, density: f32) {
        if surface_height.is_finite() {
            self.plane = Some(Plane {
                surface_height,
                density,
            });
        } else {
            self.plane = None;
        }
    }

    /// Register a bounded AABB water volume. `min`/`max` are world-space
    /// corners (the surface is the top face, `max.y`). `linear_drag` is stored
    /// as authoring data (see [`Aabb::linear_drag`]).
    pub fn add_aabb(&mut self, min: Vec3, max: Vec3, density: f32, linear_drag: f32) {
        // Normalize so min ≤ max componentwise; a caller passing them swapped
        // should still get a valid box rather than an empty one.
        let lo = min.min(max);
        let hi = min.max(max);
        self.aabbs.push(Aabb {
            min: lo,
            max: hi,
            density,
            linear_drag,
        });
    }

    /// Remove all water: both the plane and every registered AABB volume.
    pub fn clear(&mut self) {
        self.plane = None;
        self.aabbs.clear();
    }

    /// True if any water (plane or volume) is registered.
    pub fn has_any(&self) -> bool {
        self.plane.is_some() || !self.aabbs.is_empty()
    }

    /// Number of registered AABB volumes (excludes the plane). Diagnostics.
    pub fn aabb_count(&self) -> usize {
        self.aabbs.len()
    }
}

impl WaterQuery for AnalyticWater {
    fn sample(&self, point: Vec3) -> Option<WaterSample> {
        // Topmost-surface-wins, ties by registration order. Deterministic:
        // pure f32 comparison over a fixed-order set (plane, then aabbs in
        // push order); strict `>` keeps the first-registered on ties.
        let mut best: Option<WaterSample> = None;
        let mut consider = |surface_height: f32, density: f32| {
            let take = match best {
                None => true,
                Some(b) => surface_height > b.surface_height,
            };
            if take {
                best = Some(WaterSample {
                    surface_height,
                    density,
                });
            }
        };

        if let Some(p) = self.plane {
            consider(p.surface_height, p.density);
        }
        for a in &self.aabbs {
            if a.contains(point) {
                consider(a.surface_height(), a.density);
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: f32 = 1000.0;

    #[test]
    fn empty_backend_has_no_water() {
        let w = AnalyticWater::new();
        assert!(!w.has_any());
        assert_eq!(w.sample(Vec3::ZERO), None);
        assert_eq!(w.aabb_count(), 0);
    }

    #[test]
    fn plane_covers_everywhere_at_its_height() {
        let mut w = AnalyticWater::new();
        w.set_plane(4.0, D);
        // Infinite extent: any XZ, any Y (the caller does the depth check).
        for p in [
            Vec3::new(0.0, -100.0, 0.0),
            Vec3::new(999.0, 4.0, -999.0),
            Vec3::new(-5.0, 50.0, 5.0),
        ] {
            let s = w.sample(p).expect("plane covers all points");
            assert_eq!(s.surface_height, 4.0);
            assert_eq!(s.density, D);
        }
    }

    #[test]
    fn non_finite_plane_height_clears_plane() {
        let mut w = AnalyticWater::new();
        w.set_plane(4.0, D);
        assert!(w.has_any());
        w.set_plane(f32::NEG_INFINITY, D); // the retired "no water" sentinel
        assert!(!w.has_any());
        assert_eq!(w.sample(Vec3::ZERO), None);
    }

    #[test]
    fn aabb_contains_only_within_the_box() {
        let mut w = AnalyticWater::new();
        w.add_aabb(Vec3::new(-2.0, 0.0, -2.0), Vec3::new(2.0, 3.0, 2.0), D, 0.5);
        // Inside → sample with surface at top face (y=3).
        let s = w.sample(Vec3::new(0.0, 1.0, 0.0)).unwrap();
        assert_eq!(s.surface_height, 3.0);
        // Outside laterally → None.
        assert_eq!(w.sample(Vec3::new(5.0, 1.0, 0.0)), None);
        // Below the box floor → None (beneath the pool, not in it).
        assert_eq!(w.sample(Vec3::new(0.0, -1.0, 0.0)), None);
        // Above the surface but inside the box's y-range top → covered
        // (caller decides submersion); above max.y → None.
        assert!(w.sample(Vec3::new(0.0, 3.0, 0.0)).is_some());
        assert_eq!(w.sample(Vec3::new(0.0, 4.0, 0.0)), None);
    }

    #[test]
    fn swapped_corners_normalize() {
        let mut w = AnalyticWater::new();
        // max/min passed swapped — must still produce a valid box.
        w.add_aabb(Vec3::new(2.0, 3.0, 2.0), Vec3::new(-2.0, 0.0, -2.0), D, 0.0);
        assert!(w.sample(Vec3::new(0.0, 1.0, 0.0)).is_some());
    }

    #[test]
    fn overlap_resolution_topmost_surface_wins() {
        let mut w = AnalyticWater::new();
        // Low pool (surface y=2) and a high pool (surface y=5) overlapping at origin.
        w.add_aabb(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 2.0, 5.0), 1000.0, 0.0);
        w.add_aabb(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 5.0, 5.0), 1025.0, 0.0);
        let s = w.sample(Vec3::new(0.0, 1.0, 0.0)).unwrap();
        assert_eq!(s.surface_height, 5.0, "topmost surface wins");
        assert_eq!(s.density, 1025.0, "and its density");
    }

    #[test]
    fn overlap_tie_breaks_by_registration_order() {
        let mut w = AnalyticWater::new();
        // Two volumes with identical surface height; first registered wins.
        w.add_aabb(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 3.0, 5.0), 1000.0, 0.0);
        w.add_aabb(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 3.0, 5.0), 1025.0, 0.0);
        let s = w.sample(Vec3::new(0.0, 1.0, 0.0)).unwrap();
        assert_eq!(s.density, 1000.0, "first-registered wins the tie");
    }

    #[test]
    fn plane_and_aabb_combine_topmost_wins() {
        let mut w = AnalyticWater::new();
        w.set_plane(1.0, 1000.0); // low global sea
        w.add_aabb(Vec3::new(-2.0, 0.0, -2.0), Vec3::new(2.0, 6.0, 2.0), 1025.0, 0.0); // raised tank
        // Inside the tank, the tank (y=6) beats the plane (y=1).
        let inside = w.sample(Vec3::new(0.0, 3.0, 0.0)).unwrap();
        assert_eq!(inside.surface_height, 6.0);
        // Outside the tank, only the plane covers.
        let outside = w.sample(Vec3::new(50.0, 0.0, 50.0)).unwrap();
        assert_eq!(outside.surface_height, 1.0);
    }

    /// Gate Q1 determinism guarantee, made an enforced test: two independently
    /// built backends with the same volumes yield bit-identical samples, and
    /// the result is independent of query order.
    #[test]
    fn determinism_identical_backends_and_order_independence() {
        let build = || {
            let mut w = AnalyticWater::new();
            w.set_plane(3.5, 1000.0);
            w.add_aabb(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 7.25, 4.0), 1013.0, 0.3);
            w.add_aabb(Vec3::new(10.0, 1.0, 10.0), Vec3::new(20.0, 2.5, 20.0), 998.0, 0.1);
            w
        };
        let a = build();
        let b = build();

        let probes = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(15.0, 2.0, 15.0),
            Vec3::new(100.0, -3.0, 100.0),
            Vec3::new(3.99, 7.0, -3.99),
            Vec3::new(-50.0, 0.0, 0.0),
        ];

        // Two independent backends agree bit-for-bit.
        for p in probes {
            assert_eq!(a.sample(p), b.sample(p), "backends diverged at {p:?}");
        }

        // Order independence: sampling forward vs reversed yields the same map.
        let fwd: Vec<_> = probes.iter().map(|p| a.sample(*p)).collect();
        let rev: Vec<_> = probes.iter().rev().map(|p| a.sample(*p)).collect();
        let rev_realigned: Vec<_> = rev.into_iter().rev().collect();
        assert_eq!(fwd, rev_realigned, "sample result depends on query order");
    }
}
