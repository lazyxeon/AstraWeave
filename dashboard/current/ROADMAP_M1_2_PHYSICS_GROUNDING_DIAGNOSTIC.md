# M1.2 — Physics Grounding Regression: Diagnostic Recon (verdict)

> **Campaign**: R-series · **Phase**: M1.2 recon (first M1 execution beat) · **Branch**: `campaign/roadmap`
> **Mode**: READ-ONLY DIAGNOSTIC — diagnose the version-drift-vs-config fork with evidence; **no fix applied**.
> **Input**: the R.1 roadmap M1 physics item; R.0.B verdict `astraweave-physics` PRODUCTION-CAPABLE-FAILING-TESTS (1693/1).
> **Status**: VERDICT RENDERED — awaiting ratification before the M1.2-fix beat.
> **Date**: 2026-06-30

---

## Verdict (headline)

**Fork (b) — wrapper-config/logic bug. Decisively NOT Rapier-version-drift.** And a framing correction: **there is no Rapier `KinematicCharacterController` integration at all** — `astraweave-physics`'s character controller is a **fully custom manual implementation** that uses Rapier only for the rigid body, the capsule collider, and downward raycasting. The grounding bug is in AstraWeave's own snap arithmetic, which ignores the capsule's half-extent. **Blast radius: one function in one crate** (`control_character`) — no workspace `rapier3d` version change, no other crate affected. The broad-radius fork (a) is ruled out.

---

## 1. The failing test — exact assertion + setup

**`character_controller_stays_on_ground`** — `astraweave-physics/tests/physics_core_tests.rs:266-286` (panic at `:281`).

```rust
let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.8, 0.0));
let _ground = world.create_ground_plane(Vec3::new(10.0, 0.5, 10.0), 0.9);
let char_id  = world.add_character(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.4, 0.9, 0.4));
for _ in 0..60 { world.control_character(char_id, Vec3::ZERO, 1.0/60.0, false); world.step(); }
let final_y = world.body_transform(char_id).unwrap().w_axis.y;   // capsule CENTER y
assert!(final_y > 0.5 && final_y < 2.0, "Character should stay on ground, y={}", final_y);
```

- **Capsule:** `add_character` builds `capsule_y(half.y=0.9, radius=0.4)` → a Rapier capsule **centered on the rigid-body translation**, extending ±(0.9 + 0.4) = **±1.3** in Y. So `body_transform(...).y` is the capsule **center**.
- **Ground surface:** `create_ground_plane(Vec3(10,0.5,10))` builds its collider as `cuboid(half.x, 0.1, half.z)` — **the y half-extent is hardcoded to `0.1`, the caller's `half.y=0.5` is ignored** (lib.rs:1153). Fixed body at origin ⇒ **ground top surface at y = 0.1.**
- **Expected vs observed:** the test expects the capsule center in `(0.5, 2.0)` (i.e. resting on the surface ⇒ center ≈ surface + 1.3 ≈ **1.4**). **Observed `final_y = 0.1`** (the assert at :281 fails) — the capsule center sits *on the ground surface*, fully buried.

## 2. The wrapper's actual configuration (it is custom, not Rapier's controller)

- `add_character` (`src/lib.rs:1210`): `RigidBodyBuilder::kinematic_position_based()` + `ColliderBuilder::capsule_y(half.y, half.x.max(half.z))`. Stores a `CharacterController` with `height = half.y*2.0 = 1.8`, `radius = 0.4`, `max_step = 0.4`.
- `control_character` (`src/lib.rs:1258`): **manual** — applies gravity to `vertical_velocity`, does manual jump/coyote logic, a manual forward raycast for obstacle slide, and a **manual downward raycast ground-snap**:
  ```
  cast_origin = new_pos + Y*ctrl.height            // 1.8 above the tentative position
  ground_y    = cast_origin.y - hit.time_of_impact // = the ground SURFACE y
  if new_pos.y <= ground_y + 0.05 { new_pos.y = ground_y; … }   // src/lib.rs:1360-1361
  ```
- **The bug:** the snap sets `new_pos.y` (the capsule **center** / rigid-body translation) to `ground_y` (the ground **surface**). It treats the capsule's center as if it were its bottom — **it never adds the capsule half-extent (`height/2 + radius = 1.3`).** So the resting center = surface y = **0.1**, exactly the symptom. Both the snap *target* (1361) and the snap *condition* (1360, `new_pos.y <= ground_y + 0.05`) share the same center-as-bottom error.
- **No Rapier character controller anywhere:** `rg 'KinematicCharacterController|move_shape|snap_to_ground|autostep'` over `astraweave-physics/src/` ⇒ **zero hits.** There is no Rapier controller offset/snap behavior in play to have drifted.

## 3. Rapier version + drift check (fork a — ruled out)

- **Pin:** `rapier3d = { version = "0.22", … }` (workspace `Cargo.toml:195`); resolved **0.22.0** (`Cargo.lock`). `astraweave-physics` uses `{ workspace = true }`.
- **History:** `git log -S "rapier3d" -- Cargo.toml` ⇒ a single hit, `97ab6bf6d` ("Add new dependencies") — the **initial add**, *not* a version bump. rapier3d has been 0.22 throughout. The snap logic (`new_pos.y = ground_y`) was introduced in `bd19b832b` and **has not changed since**.
- **Conclusion:** no Rapier version change correlates with the failure, and the failing code path makes no Rapier-character-controller call. Fork (a) is excluded on three independent grounds: (i) no Rapier controller is used; (ii) the bug is AstraWeave's own arithmetic; (iii) no `rapier3d` version bump exists.

## 4. Regression-vs-long-standing (a framing correction)

The roadmap called this a "regression." The evidence points instead to a **long-standing wrapper-logic bug** exposed by a coverage-push test (the test entered in `0e987e676`, a coverage milestone; the snap logic predates it and is unchanged; no Rapier drift). The "version-drift-vs-config fork" resolves cleanly to **config**, and more deeply the "Rapier `KinematicCharacterController` integration" framing was itself inaccurate — it is a custom manual controller. *(This does not change the verdict `PRODUCTION-CAPABLE-FAILING-TESTS`; the crate still has one failing test until the fix lands.)*

## 5. Proposed fix (for the M1.2-fix beat — NOT applied here)

**Scope: `control_character` in `astraweave-physics/src/lib.rs` only. Contained to the crate. No Rapier version change, no blast radius.**

Account for the capsule half-extent in both the snap condition and the snap target. With `half_total = ctrl.height * 0.5 + ctrl.radius` (= 0.9 + 0.4 = 1.3):
- **Snap target (1361):** `new_pos.y = ground_y + half_total;` (center rests above the surface so the capsule *bottom* touches it).
- **Snap condition (1360):** compare the capsule **bottom** to the ground: `if new_pos.y - half_total <= ground_y + 0.05 { … }` (equivalently `new_pos.y <= ground_y + half_total + 0.05`).
- **Result:** resting center = `0.1 + 1.3 = 1.4` ∈ (0.5, 2.0) ⇒ the test passes; the capsule rests *on* the surface instead of buried. `astraweave-npc` (which drives `control_character`) inherits the correct grounding. Moves `physics` → VERIFIED-PRODUCTION.

### 5.1 Secondary findings (note; the fix-beat decides whether to also address)

- **`create_ground_plane` hardcodes the y half-extent to `0.1`** (`src/lib.rs:1153`), ignoring the caller's `half.y`. The test passed `half.y=0.5` expecting a 0.5-thick ground; it got 0.1. This is a *separate* latent wrapper inconsistency — not the cause of the grounding bug (the snap bug fails regardless of ground thickness) but worth a one-line fix (`cuboid(half.x, half.y, half.z)`) or a documented intent. It coincidentally supplies the literal `0.1` in the symptom.
- The fix should add an explicit **resting-y assertion** (or tighten the existing loose `(0.5, 2.0)` band to the computed `1.4`) so the convention is pinned against future drift.

---

## What this is NOT

- **NOT the fix** — no wrapper code changed, no offset tweaked, no version bumped. The M1.2-fix beat applies the §5 fix after ratification.
- **NOT the other M1 items** — test-rot, A2, trace-honesty are later in M1.
- **NOT a verdict change** — `physics` stays PRODUCTION-CAPABLE-FAILING-TESTS until the fix lands.

*Read-only diagnostic. Tree unchanged (the test run mutated no fixtures — post-run `git status` clean). Awaiting ratification of the fork verdict before the M1.2-fix beat.*
