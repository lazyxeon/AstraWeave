//! F.4.2 binary-glue weave-impact accent producer (A2 — CPU producer).
//!
//! Sibling to [`WaterWeaveProducer`](crate::weave_producer::WaterWeaveProducer):
//! both are fed the same applied [`WeaveOp`]s. Where the weave producer drives
//! the *surface deformation* (part/raise/freeze), THIS producer drives the
//! *splash/spray accents* the weave throws — the fate-weaving accent beat.
//!
//! It mirrors the weave producer's pattern exactly: it owns the accent
//! particles' lifetime on the CPU, ballistically ages them each frame, and
//! snapshots the live set as [`SecondaryParticle`]s for upload via
//! [`FluidSystem::set_secondary_particles`]. The renderer holds no accent
//! lifetime state. This lives in the binary because it is the one place that
//! legitimately knows BOTH `astraweave-gameplay` (`WeaveOp`) and
//! `astraweave-fluids` (`SecondaryParticle`) — neither engine crate depends on
//! the other; the translation is one-directional (gameplay → render accents).
//!
//! Ratified style (F.4.1 gate): A2 CPU producer, weave-impact ONLY (crest /
//! shoreline deferred), motion **B2 floaty/magical** (low gravity, long life,
//! slow fade), shape/colour **A1+C3** carried per-kind in `info.y` and rendered
//! by `secondary.wgsl`. The motion/spawn parameters here ARE the art-directable
//! surface for feel; the shader's tint/shape LUT is the surface for look.
//!
//! [`WeaveOp`]: astraweave_gameplay::WeaveOp
//! [`SecondaryParticle`]: astraweave_fluids::SecondaryParticle
//! [`FluidSystem::set_secondary_particles`]: astraweave_fluids::FluidSystem::set_secondary_particles

use astraweave_fluids::SecondaryParticle;
use astraweave_gameplay::{WeaveOp, WeaveOpKind};
use glam::{Vec2, Vec3};

/// Gravity (m/s²) applied to accent particles, scaled per-kind by `gravity_scale`.
const GRAVITY: f32 = -9.81;
/// Hard cap on live accent particles (the F.4.1 budget is ~1–4k; we stay well
/// under it). Excess spawns drop the oldest.
const MAX_ACCENT_PARTICLES: usize = 2048;

/// Accent kind index — packed into `SecondaryParticle.info.y` for the shader's
/// per-kind tint + shape LUT. Must match `secondary.wgsl`'s `tint_for_kind` /
/// `shape_mask` (0=Part, 1=Raise, 2=Freeze).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum AccentKind {
    Part = 0,
    Raise = 1,
    Freeze = 2,
}

impl AccentKind {
    fn index(self) -> f32 {
        self as i32 as f32
    }
}

/// Map a gameplay op kind to an accent kind. Only the three water weave ops
/// produce accents; everything else (terrain/weather + future `#[non_exhaustive]`
/// variants) produces none — matching the weave producer's `render_kind`.
fn accent_kind(op_kind: WeaveOpKind) -> Option<AccentKind> {
    match op_kind {
        WeaveOpKind::LowerWater => Some(AccentKind::Part),
        WeaveOpKind::RaisePlatform => Some(AccentKind::Raise),
        WeaveOpKind::FreezeWater => Some(AccentKind::Freeze),
        _ => None,
    }
}

/// Per-kind tunable style (motion + spawn). B2 floaty/magical defaults.
#[derive(Copy, Clone)]
struct AccentStyle {
    /// Initial speed of spawned particles (m/s).
    speed: f32,
    /// Lateral spread of the spawn velocity cone (m/s).
    spread: f32,
    /// Gravity multiplier — < 1.0 reads floaty/suspended (B2).
    gravity_scale: f32,
    /// Per-particle lifetime (s).
    particle_lifetime: f32,
    /// Ramp-in / fade-out seconds at each end of the alpha envelope.
    ramp: f32,
    /// Billboard world-space scale.
    scale: f32,
    /// Continuous spawn rate (particles / second) while the emitter is alive.
    spawn_rate: f32,
    /// Seconds the emitter keeps spawning (continuous kinds). Ignored for one-shot.
    emitter_lifetime: f32,
    /// One-shot: emit a single burst then stop (Freeze — honours the surface's
    /// `(1-freeze)` foam suppression: frost shimmers once, then the frozen
    /// surface goes quiet).
    one_shot: bool,
    /// Particles emitted in a one-shot burst.
    burst: u32,
}

fn style_for(kind: AccentKind) -> AccentStyle {
    match kind {
        // Raise → upward lift-burst, clean and buoyant.
        AccentKind::Raise => AccentStyle {
            speed: 4.5,
            spread: 1.6,
            gravity_scale: 0.35,
            particle_lifetime: 1.6,
            ramp: 0.25,
            scale: 0.5,
            spawn_rate: 90.0,
            emitter_lifetime: 1.2,
            one_shot: false,
            burst: 0,
        },
        // Part → outward + down silt spray (water shoved aside, bed exposed).
        AccentKind::Part => AccentStyle {
            speed: 3.8,
            spread: 2.4,
            gravity_scale: 0.6,
            particle_lifetime: 1.3,
            ramp: 0.2,
            scale: 0.45,
            spawn_rate: 80.0,
            emitter_lifetime: 1.0,
            one_shot: false,
            burst: 0,
        },
        // Freeze → single frost shimmer, then suppress.
        AccentKind::Freeze => AccentStyle {
            speed: 2.2,
            spread: 1.2,
            gravity_scale: 0.15,
            particle_lifetime: 2.2,
            ramp: 0.4,
            scale: 0.55,
            spawn_rate: 0.0,
            emitter_lifetime: 0.0,
            one_shot: true,
            burst: 120,
        },
    }
}

/// A live spawn source created from one applied weave op.
struct AccentEmitter {
    kind: AccentKind,
    position: Vec2, // world XZ
    age: f32,
    style: AccentStyle,
    /// Fractional spawn carry for sub-particle-per-frame rates.
    spawn_accum: f32,
    /// One-shot burst already fired?
    fired: bool,
}

/// One live accent particle, aged ballistically on the CPU.
struct AccentParticle {
    position: Vec3,
    velocity: Vec3,
    age: f32,
    lifetime: f32,
    ramp: f32,
    scale: f32,
    kind: AccentKind,
}

impl AccentParticle {
    /// Alpha envelope 0..1: ramp-in → hold → fade-out (B2 slow fade).
    fn alpha(&self) -> f32 {
        if self.ramp <= 0.0 || self.lifetime <= 0.0 {
            return 1.0;
        }
        let a = if self.age < self.ramp {
            self.age / self.ramp
        } else if self.age > self.lifetime - self.ramp {
            (self.lifetime - self.age) / self.ramp
        } else {
            1.0
        };
        a.clamp(0.0, 1.0)
    }
}

/// The accent producer: owns emitters + the live particle pool, ages both,
/// snapshots the live set.
pub struct WaterAccentProducer {
    emitters: Vec<AccentEmitter>,
    particles: Vec<AccentParticle>,
    /// Y the accents spawn at (the water surface level). Tunable.
    water_level: f32,
    /// Deterministic PRNG state (xorshift) — keeps tests reproducible and avoids
    /// a `rand` dependency.
    rng: u32,
}

impl Default for WaterAccentProducer {
    fn default() -> Self {
        Self {
            emitters: Vec::new(),
            particles: Vec::new(),
            water_level: 0.0,
            rng: 0x9E37_79B9, // golden-ratio seed (nonzero)
        }
    }
}

impl WaterAccentProducer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the world-space Y the accents spawn from (the water surface). Tunable.
    #[allow(dead_code)] // wired by F.4.3 demo integration; a setter otherwise
    pub fn set_water_level(&mut self, y: f32) {
        self.water_level = y;
    }

    fn next_f32(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        ((x >> 8) as f32) / ((1u32 << 24) as f32) // [0,1)
    }

    /// Signed jitter in [-1, 1).
    fn jitter(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }

    /// Translate an applied `WeaveOp` into an accent emitter. Non-water ops are
    /// ignored (mirrors the weave producer). Same call site as
    /// `WaterWeaveProducer::ingest`.
    pub fn ingest(&mut self, op: &WeaveOp) {
        let Some(kind) = accent_kind(op.kind) else {
            return;
        };
        self.emitters.push(AccentEmitter {
            kind,
            position: Vec2::new(op.a.x, op.a.z), // Vec3 → world XZ
            age: 0.0,
            style: style_for(kind),
            spawn_accum: 0.0,
            fired: false,
        });
    }

    /// Spawn one particle from an emitter, with per-kind initial velocity.
    fn spawn_one(&mut self, kind: AccentKind, origin: Vec2, style: AccentStyle) {
        if self.particles.len() >= MAX_ACCENT_PARTICLES {
            self.particles.remove(0); // budget cap: drop the oldest
        }
        let jx = self.jitter();
        let jz = self.jitter();
        let pos = Vec3::new(origin.x + jx * 0.5, self.water_level, origin.y + jz * 0.5);
        let velocity = match kind {
            // Upward lift-burst with lateral spread.
            AccentKind::Raise => Vec3::new(
                jx * style.spread,
                style.speed * (0.8 + 0.4 * self.next_f32()),
                jz * style.spread,
            ),
            // Outward + slightly down (water shoved aside).
            AccentKind::Part => {
                let dir = Vec2::new(jx, jz).normalize_or_zero();
                Vec3::new(
                    dir.x * style.speed,
                    -style.speed * 0.25,
                    dir.y * style.speed,
                )
            }
            // Gentle, near-suspended frost rise.
            AccentKind::Freeze => Vec3::new(
                jx * style.spread,
                style.speed * (0.3 + 0.3 * self.next_f32()),
                jz * style.spread,
            ),
        };
        self.particles.push(AccentParticle {
            position: pos,
            velocity,
            age: 0.0,
            lifetime: style.particle_lifetime,
            ramp: style.ramp,
            scale: style.scale,
            kind,
        });
    }

    /// Advance emitters (spawning) and particles (ballistic aging) by `dt`.
    pub fn tick(&mut self, dt: f32) {
        // 1. Emit from each live emitter.
        //    Collect spawn requests first (avoid borrow conflict with spawn_one).
        let mut to_spawn: Vec<(AccentKind, Vec2, AccentStyle)> = Vec::new();
        for e in &mut self.emitters {
            e.age += dt;
            let s = e.style;
            if s.one_shot {
                if !e.fired {
                    e.fired = true;
                    for _ in 0..s.burst {
                        to_spawn.push((e.kind, e.position, s));
                    }
                }
            } else if e.age < s.emitter_lifetime {
                e.spawn_accum += s.spawn_rate * dt;
                while e.spawn_accum >= 1.0 {
                    e.spawn_accum -= 1.0;
                    to_spawn.push((e.kind, e.position, s));
                }
            }
        }
        for (kind, origin, style) in to_spawn {
            self.spawn_one(kind, origin, style);
        }

        // 2. Drop spent emitters (one-shot fired, or continuous past lifetime).
        self.emitters.retain(|e| {
            if e.style.one_shot {
                !e.fired
            } else {
                e.age < e.style.emitter_lifetime
            }
        });

        // 3. Ballistic age the particle pool (B2 floaty: gravity_scale < 1).
        for p in &mut self.particles {
            let g = GRAVITY * style_for(p.kind).gravity_scale;
            p.velocity.y += g * dt;
            p.position += p.velocity * dt;
            p.age += dt;
        }
        self.particles.retain(|p| p.age < p.lifetime);
    }

    /// The live accent set as renderable `SecondaryParticle`s (≤ budget). Push
    /// this to `FluidSystem::set_secondary_particles` each frame; expired
    /// particles are simply absent (the setter replaces the whole list).
    ///
    /// `info = (age, kind_index, alpha_envelope, scale)` — the shader reads
    /// `kind`, `alpha`, `scale`; `age` is carried for any future shader use.
    #[allow(dead_code)] // consumed by F.4.3 demo upload; exercised by tests here
    pub fn snapshot(&self) -> Vec<SecondaryParticle> {
        self.particles
            .iter()
            .take(MAX_ACCENT_PARTICLES)
            .map(|p| SecondaryParticle {
                position: [p.position.x, p.position.y, p.position.z, 1.0],
                velocity: [p.velocity.x, p.velocity.y, p.velocity.z, 0.0],
                info: [p.age, p.kind.index(), p.alpha(), p.scale],
            })
            .collect()
    }

    /// Number of live accent particles (diagnostics / tests).
    #[allow(dead_code)] // consumed by F.4.3 demo upload; exercised by tests here
    pub fn live_count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;

    fn op(kind: WeaveOpKind, x: f32, z: f32) -> WeaveOp {
        WeaveOp {
            kind,
            a: vec3(x, 1.5, z),
            b: None,
            budget_cost: 1,
        }
    }

    #[test]
    fn zero_state_is_empty() {
        // Zero-accent identity: a fresh producer renders nothing.
        let p = WaterAccentProducer::new();
        assert_eq!(p.live_count(), 0);
        assert!(p.snapshot().is_empty());
    }

    #[test]
    fn translates_three_water_ops_to_emitters() {
        let mut p = WaterAccentProducer::new();
        p.ingest(&op(WeaveOpKind::LowerWater, 10.0, -4.0)); // Part
        p.ingest(&op(WeaveOpKind::RaisePlatform, 20.0, 5.0)); // Raise
        p.ingest(&op(WeaveOpKind::FreezeWater, -7.0, 3.0)); // Freeze
        assert_eq!(p.emitters.len(), 3);
        assert_eq!(p.emitters[0].kind, AccentKind::Part);
        assert_eq!(p.emitters[1].kind, AccentKind::Raise);
        assert_eq!(p.emitters[2].kind, AccentKind::Freeze);
        // Location comes ONLY from op.a.xz (y dropped).
        assert_eq!(p.emitters[0].position, Vec2::new(10.0, -4.0));
    }

    #[test]
    fn non_water_ops_produce_no_accents() {
        let mut p = WaterAccentProducer::new();
        p.ingest(&op(WeaveOpKind::ReinforcePath, 0.0, 0.0));
        p.ingest(&op(WeaveOpKind::CollapseBridge, 0.0, 0.0));
        p.ingest(&op(WeaveOpKind::RedirectWind, 0.0, 0.0));
        p.tick(0.1);
        assert_eq!(p.emitters.len(), 0);
        assert_eq!(p.live_count(), 0);
        assert!(p.snapshot().is_empty());
    }

    #[test]
    fn continuous_emitter_spawns_particles_with_kind() {
        let mut p = WaterAccentProducer::new();
        p.ingest(&op(WeaveOpKind::RaisePlatform, 0.0, 0.0)); // Raise: continuous
        p.tick(0.1);
        assert!(p.live_count() > 0, "raise emitter should spawn spray");
        let snap = p.snapshot();
        assert!(!snap.is_empty());
        // info.y carries the Raise kind index (1.0).
        assert_eq!(snap[0].info[1], AccentKind::Raise.index());
        // alpha (info.z) is within the envelope, scale (info.w) is the style scale.
        assert!(snap[0].info[2] >= 0.0 && snap[0].info[2] <= 1.0);
        assert!(snap[0].info[3] > 0.0);
    }

    #[test]
    fn freeze_is_one_shot_burst_then_suppresses() {
        let mut p = WaterAccentProducer::new();
        p.ingest(&op(WeaveOpKind::FreezeWater, 0.0, 0.0)); // Freeze: one-shot
        p.tick(0.016); // fire the burst
        let after_burst = p.live_count();
        assert!(
            after_burst > 0,
            "freeze should fire a one-shot shimmer burst"
        );
        // The emitter must be spent after firing (no continued emission).
        assert_eq!(p.emitters.len(), 0, "one-shot freeze emitter must retire");
        // A subsequent tick spawns NO new freeze particles (count only decays).
        p.tick(0.016);
        assert!(
            p.live_count() <= after_burst,
            "frozen surface stays quiet after the shimmer"
        );
    }

    #[test]
    fn particles_expire_after_lifetime() {
        let mut p = WaterAccentProducer::new();
        p.ingest(&op(WeaveOpKind::RaisePlatform, 0.0, 0.0));
        p.tick(0.1);
        assert!(p.live_count() > 0);
        // Advance well past the emitter lifetime AND every particle lifetime.
        for _ in 0..40 {
            p.tick(0.1); // 4.0 s total ≫ emitter 1.2 s + particle 1.6 s
        }
        assert_eq!(p.live_count(), 0, "all accents should expire");
        assert!(p.snapshot().is_empty());
    }

    #[test]
    fn budget_caps_live_particles() {
        let mut p = WaterAccentProducer::new();
        // Many simultaneous freeze bursts (120 each) overrun the budget.
        for i in 0..40 {
            p.ingest(&op(WeaveOpKind::FreezeWater, i as f32, 0.0));
        }
        p.tick(0.016);
        assert!(
            p.live_count() <= MAX_ACCENT_PARTICLES,
            "live particles must stay within the budget cap"
        );
        assert!(p.snapshot().len() <= MAX_ACCENT_PARTICLES);
    }
}
