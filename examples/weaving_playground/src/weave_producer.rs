//! W.2c.3 binary-glue weave producer.
//!
//! Translates gameplay [`WeaveOp`]s into render-side [`WeaveInstance`]s, ages each
//! through a synthetic lifetime envelope (so an *instantaneous* op gets a visible
//! duration), and snapshots the live set for [`Renderer::set_water_weave_instances`].
//!
//! This lives in the **binary glue** because it is the one place that legitimately
//! knows BOTH `astraweave-gameplay` (`WeaveOp`/`WeaveOpKind`) and `astraweave-render`
//! (`WeaveInstance`/`WeaveKind`). Neither engine crate depends on the other — the
//! translation is one-directional (gameplay → render) and confined here.
//!
//! Ratified mapping (W.2c.3.1 gate): `LowerWater → Part`, `RaisePlatform → Raise`,
//! `FreezeWater → Freeze`; terrain/weather ops produce no water weave. Location
//! comes ONLY from `op.a` (projected to world XZ) — the analytical profile is
//! position-agnostic. Intensity/lifetime are per-kind defaults + the envelope
//! (ratified option (a)); the ability-sourced path stays deferred.
//!
//! [`WeaveOp`]: astraweave_gameplay::WeaveOp
//! [`WeaveInstance`]: astraweave_render::WeaveInstance
//! [`Renderer::set_water_weave_instances`]: astraweave_render::Renderer::set_water_weave_instances

use astraweave_gameplay::{WeaveOp, WeaveOpKind};
use astraweave_render::{WeaveInstance, WeaveKind, MAX_WEAVE_INSTANCES};
use glam::Vec2;

/// Per-kind presentation defaults. Intensities match the W.2c.2 editor scaffolding
/// so the runtime and editor read consistently.
struct WeaveDefaults {
    base_intensity: f32,
    radius: f32,
    /// Total seconds the render weave lives (the instantaneous op's visible duration).
    lifetime: f32,
    /// Ramp-in / fade-out seconds at each end of the envelope.
    ramp: f32,
}

fn defaults_for(kind: WeaveKind) -> WeaveDefaults {
    match kind {
        WeaveKind::Part => WeaveDefaults { base_intensity: 0.7, radius: 30.0, lifetime: 4.0, ramp: 0.5 },
        WeaveKind::Raise => WeaveDefaults { base_intensity: 0.6, radius: 30.0, lifetime: 4.0, ramp: 0.5 },
        WeaveKind::Freeze => WeaveDefaults { base_intensity: 1.0, radius: 30.0, lifetime: 6.0, ramp: 0.8 },
        WeaveKind::None => WeaveDefaults { base_intensity: 0.0, radius: 1.0, lifetime: 0.0, ramp: 0.0 },
    }
}

/// Map a gameplay op kind to a render weave kind. Only the three ratified water ops
/// translate; the `_` arm absorbs the terrain/weather ops AND any future
/// `#[non_exhaustive]` variant (which simply produces no weave until mapped).
fn render_kind(op_kind: WeaveOpKind) -> Option<WeaveKind> {
    match op_kind {
        WeaveOpKind::LowerWater => Some(WeaveKind::Part),
        WeaveOpKind::RaisePlatform => Some(WeaveKind::Raise),
        WeaveOpKind::FreezeWater => Some(WeaveKind::Freeze),
        _ => None,
    }
}

/// One live render weave plus its age (drives the envelope).
struct ActiveWeave {
    kind: WeaveKind,
    position: Vec2,
    radius: f32,
    base_intensity: f32,
    lifetime: f32,
    ramp: f32,
    age: f32,
}

impl ActiveWeave {
    /// Envelope 0..1: ramp-in → hold → fade-out. Modulates `intensity` (the shader
    /// consumes intensity for the deformation magnitude, so the envelope is what
    /// gives the instantaneous op a visible ramped duration).
    fn envelope(&self) -> f32 {
        if self.ramp <= 0.0 || self.lifetime <= 0.0 {
            return 1.0;
        }
        let e = if self.age < self.ramp {
            self.age / self.ramp
        } else if self.age > self.lifetime - self.ramp {
            (self.lifetime - self.age) / self.ramp
        } else {
            1.0
        };
        e.clamp(0.0, 1.0)
    }
}

/// The producer: owns the active weave set, ages it, and snapshots the live set.
#[derive(Default)]
pub struct WaterWeaveProducer {
    active: Vec<ActiveWeave>,
}

impl WaterWeaveProducer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Translate a gameplay `WeaveOp` into a render weave and add it. Non-water ops
    /// are ignored. Caps the live set at [`MAX_WEAVE_INSTANCES`] (drops the oldest).
    pub fn ingest(&mut self, op: &WeaveOp) {
        let Some(kind) = render_kind(op.kind) else {
            return;
        };
        let d = defaults_for(kind);
        self.active.push(ActiveWeave {
            kind,
            position: Vec2::new(op.a.x, op.a.z), // Vec3 → world XZ
            radius: d.radius,
            base_intensity: d.base_intensity,
            lifetime: d.lifetime,
            ramp: d.ramp,
            age: 0.0,
        });
        while self.active.len() > MAX_WEAVE_INSTANCES {
            self.active.remove(0);
        }
    }

    /// Advance every active weave by `dt` and drop those whose envelope completed.
    pub fn tick(&mut self, dt: f32) {
        for w in &mut self.active {
            w.age += dt;
        }
        self.active.retain(|w| w.age < w.lifetime);
    }

    /// The live render set (≤ [`MAX_WEAVE_INSTANCES`]), envelope-modulated. Push this
    /// to `Renderer::set_water_weave_instances` each frame; expired weaves are simply
    /// absent (the setter replaces the whole list).
    pub fn snapshot(&self) -> Vec<WeaveInstance> {
        self.active
            .iter()
            .rev() // newest first if we ever exceed the ceiling
            .take(MAX_WEAVE_INSTANCES)
            .map(|w| WeaveInstance {
                kind: w.kind,
                position: w.position,
                radius: w.radius,
                orientation: 0.0, // single-point ops → default orientation (W.2c.3.1)
                intensity: w.base_intensity * w.envelope(),
                phase: w.age, // raw age carried for future shader-side animation
            })
            .collect()
    }

    /// Number of live weaves (diagnostics / tests).
    #[allow(dead_code)] // exercised by the unit tests; a diagnostic accessor otherwise
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;

    fn op(kind: WeaveOpKind, x: f32, z: f32) -> WeaveOp {
        WeaveOp { kind, a: vec3(x, 1.5, z), b: None, budget_cost: 1 }
    }

    #[test]
    fn translates_three_water_ops_to_instances() {
        let mut p = WaterWeaveProducer::new();
        p.ingest(&op(WeaveOpKind::LowerWater, 10.0, -4.0));
        p.ingest(&op(WeaveOpKind::RaisePlatform, 20.0, 5.0));
        p.ingest(&op(WeaveOpKind::FreezeWater, -7.0, 3.0));
        p.tick(0.5); // advance into the ramp so the envelope is > 0
        let snap = p.snapshot();
        assert_eq!(snap.len(), 3);
        // snapshot is newest-first: freeze, raise, part
        assert_eq!(snap[0].kind, WeaveKind::Freeze);
        assert_eq!(snap[1].kind, WeaveKind::Raise);
        assert_eq!(snap[2].kind, WeaveKind::Part);
        // location comes ONLY from op.a.xz (y dropped)
        assert_eq!(snap[2].position, Vec2::new(10.0, -4.0));
        assert!(snap[2].intensity > 0.0 && snap[2].intensity <= 0.7);
    }

    #[test]
    fn non_water_ops_produce_no_weave() {
        let mut p = WaterWeaveProducer::new();
        p.ingest(&op(WeaveOpKind::ReinforcePath, 0.0, 0.0));
        p.ingest(&op(WeaveOpKind::CollapseBridge, 0.0, 0.0));
        p.ingest(&op(WeaveOpKind::RedirectWind, 0.0, 0.0));
        assert_eq!(p.active_count(), 0);
        assert!(p.snapshot().is_empty());
    }

    #[test]
    fn weave_expires_after_lifetime() {
        let mut p = WaterWeaveProducer::new();
        p.ingest(&op(WeaveOpKind::LowerWater, 0.0, 0.0)); // part: lifetime 4.0
        assert_eq!(p.active_count(), 1);
        p.tick(5.0); // past lifetime
        assert_eq!(p.active_count(), 0);
        assert!(p.snapshot().is_empty());
    }

    #[test]
    fn envelope_ramps_in_holds_and_fades() {
        let mut p = WaterWeaveProducer::new();
        p.ingest(&op(WeaveOpKind::FreezeWater, 0.0, 0.0)); // lifetime 6, ramp 0.8, base 1.0
        // age 0: ramp-in just begun → ~0
        assert!(p.snapshot()[0].intensity < 0.2);
        p.tick(2.0); // age 2.0: hold → ~base
        assert!((p.snapshot()[0].intensity - 1.0).abs() < 0.01);
        p.tick(3.6); // age 5.6 > (6 - 0.8) → fading
        let fading = p.snapshot()[0].intensity;
        assert!(fading > 0.0 && fading < 1.0);
    }

    #[test]
    fn ceiling_caps_at_max() {
        let mut p = WaterWeaveProducer::new();
        for i in 0..(MAX_WEAVE_INSTANCES + 4) {
            p.ingest(&op(WeaveOpKind::LowerWater, i as f32, 0.0));
        }
        assert_eq!(p.active_count(), MAX_WEAVE_INSTANCES);
        p.tick(0.1);
        assert_eq!(p.snapshot().len(), MAX_WEAVE_INSTANCES);
    }
}
