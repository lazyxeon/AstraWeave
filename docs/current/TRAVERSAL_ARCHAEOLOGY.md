# Traversal archaeology and upgrade plan

**Status:** review gate. No traversal feature code in this change.
**Branch:** `feat/traversal-archaeology`
**Base:** `85d09a2c` (`main`, 2026-10-15)
**Survey date:** 2026-10-15

This document is the Phase 1 map and the Phase 2 plan from the traversal brief. Implementation waits for review. The research notes that informed the plan are the attached traversal research (BotW, Conan, authored parkour, swim, glide, animation). Claims below were checked against the tree at `85d09a2c` and against open PR #205 (`cursor/aravath-greybox-b3de`), which is not merged.

## How to read a status

| Status | Meaning |
|---|---|
| REAL | Implemented and called from a non-test path, or the engine step itself. |
| STUBBED | The symbol exists and its body is a placeholder, a log line, or a single special case that is not the mechanic. |
| DORMANT | Implemented and usually tested, with no non-test production caller. |
| UNTESTED | A production path with no test that locks the behavior. |
| ABSENT | No type, function, or asset for this mechanic. |

## 1. What actually moves a body

Three locomotion paths exist side by side. They do not share a state machine.

| Path | Who moves | Collision | Tick | Status |
|---|---|---|---|---|
| Kinematic capsule | `PhysicsWorld::control_character` | Rapier raycasts, not Rapier's character controller | Caller-supplied `dt` | REAL |
| Grid AI | `validate_and_execute` `MoveTo` writes `Pose.pos` | Obstacle grid, then a teleport | One plan step | REAL for the teleport; speed is ignored |
| Editor play scripts | `MovementSystem::tick_entity` | None | Editor play mode | REAL for patrol, orbit, and wander |

There is no player avatar loop on `main`. `examples/physics_demo3d` drives the capsule from raw keys and a variable frame `dt`. The Aravath demo that wires input, a motor, a third-person camera, and a locomotion graph lives only on PR #205.

## 2. Inventory

### 2.1 Character controller and physics

Rapier is `rapier3d` 0.22 (`Cargo.toml` workspace pin). `PhysicsWorld` uses `DefaultBroadPhase`. A workspace search finds no `KinematicCharacterController`.

| System | Path | Status | Notes |
|---|---|---|---|
| `CharacterController` | `astraweave-physics/src/lib.rs` 424–534 | REAL | Capsule radius, height, `max_step`, coyote and jump-buffer fields. All `f32`. |
| `CharState` | same file 392–415 | STUBBED | The only variant is `Grounded`. `control_character` never writes `state`. `is_grounded()` therefore stays true after the body leaves the ground. |
| `PhysicsWorld::add_character` | 1210–1248 | REAL | Kinematic position-based body plus a Y capsule. Inserts a controller with `max_climb_angle_deg = 70`. |
| `PhysicsWorld::control_character` | 1258–1398 | REAL | Gravity, jump consume, one forward ray slide, one downward ground ray, `set_next_kinematic_position`. |
| `PhysicsWorld::jump` | 1250–1256 | DORMANT on `main` | Sets the buffer and `pending_jump_velocity`. Call sites are tests. PR #205's motor calls it. |
| Slope limit | 1353–1374 | REAL | Rejects the snap when the ground angle exceeds `max_climb_angle_deg`. This is walkable-slope rejection, not climbing. |
| `max_step` | 429, 1349 | STUBBED as a step | The value only lengthens the downward ray. There is no step-up. |
| `_climb: bool` | 1258, 1277–1281, 1330–1333 | STUBBED | Zeroes gravity and adds `2.0 * dt` to Y. Comment: "Simple vertical climb". `physics_demo3d` can pass the flag. Tests cover the flag. |
| Coyote and jump buffer | 435–487 and 1274–1290 | REAL inside the move, UNTESTED as tick windows | Limits are seconds (`0.10` on `add_character`, `0.15` on `CharacterController::new`). The move path inlines the compares and does not call `can_jump`. |
| Wall slide | 1302–1324 | REAL | One forward ray, then project the step onto the hit plane. Not a wallrun. |
| `char_map` | 916 | REAL | `HashMap<BodyId, CharacterController>`. Keyed lookup is stable for one body. |
| Rapier step | `step` / `step_internal`, default `time_step = 1/60` | REAL | `PhysicsConfig` default is fixed. Callers can pass any `dt` into `control_character` separately from the Rapier step. |
| Buoyancy | `add_buoyancy` 1433, `apply_buoyancy_forces` 1438–1487 | REAL | Samples `astraweave_water::WaterQuery`. Impulse is `force * integration.dt`. |
| `add_water_aabb` | 1493–1495 | REAL | Registers an AABB on the water facade. The physics trace still describes this function as an empty stub (trace last verified 2026-05-12). |
| `create_ground_plane` | 1150–1163 | REAL | Collider is `cuboid(half.x, 0.1, half.z)`. The Y component of `half` is ignored. |
| `SpatialHash` | `astraweave-physics/src/spatial_hash.rs` | DORMANT | Crate docs at `lib.rs` 25–26 call it the broadphase. `PhysicsWorld` does not use it. |
| `PhysicsPlugin` | `astraweave-physics/src/ecs.rs` | DORMANT | Feature `ecs`. No production consumer enables it. |
| `process_destructible_hits` | `lib.rs` (empty body) | STUBBED | Zero callers. Already recorded in the physics trace. |
| Vehicle tire slide | `astraweave-physics/src/vehicle.rs` | DORMANT relative to the player | A friction model for wheels, not a character slide. |
| NPC bridge | `astraweave-npc/src/runtime.rs` `move_character` | REAL | Calls `control_character` with `dt = 1/60` and `_climb = false`. |
| Physics demo | `examples/physics_demo3d/src/main.rs` ~261–270 | REAL example, determinism hazard | `dt` comes from `Instant::now()`. WASD is raw keys, not `astraweave-input`. No `jump()` call. |
| Nav bridge example | `examples/nav_physics_bridge/src/main.rs` | REAL example | Fixed `dt = 1/60`. |
| Character benches | `astraweave-physics/benches/character_controller.rs` | REAL bench, known failure | `character_step_climbing` passes `_climb = true`, so it measures the vertical nudge. `character_full_tick` asserts `y > -10` inside the Criterion loop and does not reset the body. Tracked by issue #221. |
| Character tests | `astraweave-physics/tests/physics_core_tests.rs`, `spatial_hash_character_tests.rs`, `physics_laws_tests.rs` (`test_climb_parameter`), `nan_infinity_tests.rs`, `panic_safety_tests.rs`, `mutation_resistant_comprehensive_tests.rs`, inline `lib.rs` tests | REAL | `tests/determinism.rs` is `async-physics` only and uses `StdRng`, not `SimRng`. It is not in `determinism-replay.yml`. |

`handle_of` walks `body_ids` linearly (1400–1404). That is stable for a single id and becomes order-sensitive only if a later system iterates the map.

### 2.2 Climb, swim, glide, and the other requested moves

| Mechanic | Where it shows up | Status |
|---|---|---|
| Climb-any-surface | `_climb` vertical nudge only | STUBBED |
| Authored handholds | — | ABSENT |
| Climbability flags | — | ABSENT |
| Overhang / shimmy / ledge hang | — | ABSENT |
| Vault / mantle | — | ABSENT |
| Wallrun | The forward-ray slide is a deflection, not a wallrun | ABSENT as a move |
| Swim / dive / wade / oxygen / wetness | `astraweave-gameplay/src/water_movement.rs` | DORMANT |
| Water truth and buoyancy | `astraweave-water` `WaterQuery`; physics buoyancy above | REAL, not a swim controller |
| Fly | — on `main`. PR #205 `MotorMode::Fly` zeroes gravity | ABSENT on `main` |
| Glide / updraft | — | ABSENT |
| Crouch | `Action::Crouch`; `Stance::Crouch` on AI snapshots | STUBBED as locomotion. No capsule resize. |
| Prone / crawl | `Stance::Prone` in `astraweave-core/src/schema.rs` 72–76 | STUBBED vocabulary |
| Slide | Character: the wall deflection. No slope slide. | ABSENT as a traversal state |
| Roll / dodge | `ActionStep::Dodge` logs and returns (`validation.rs` 153–156) | STUBBED |
| Sprint | `MovementSpeed::Sprint` (`schema.rs` 483–487). `MoveTo` prints the speed and teleports one cell. | STUBBED |
| Stamina | `Stats.stamina: i32` (`gameplay/src/stats.rs` 5–12), default 100. Nothing in the move path reads or writes it. Water code exposes drain multipliers and nothing calls them outside tests. | DORMANT |
| I-frames | `IFrame { time_left: f32 }` on `Combatant` (`combat_physics.rs` 7–9). `perform_attack_sweep` checks `time_left > 0` and does not decrement it. | STUBBED decay. The sweep itself is REAL (`examples/combat_physics_demo`). |
| Grid approach, retreat, strafe, patrol, take cover, block, parry | `validation.rs` 48–161 | STUBBED (log only), except `MoveTo`, attacks, heal, and throw-smoke |

`WaterMovementMode` is `Dry | Wading | WaistDeep | Swimming | Diving` (`water_movement.rs` 12–24). Thresholds are submersion fractions, not a depth query with hysteresis. `WaterPlayerState::update` is deterministic given `(submersion, dt)` and is covered by mutation tests. A workspace search shows no production caller outside `astraweave-gameplay` tests.

### 2.3 Input

| System | Path | Status |
|---|---|---|
| `Action` | `astraweave-input/src/actions.rs` 46–75 | REAL |
| Movement actions | `MoveForward/Backward/Left/Right`, `Jump`, `Crouch`, `Sprint` | REAL digital actions. No `Dodge`. `Ability1` and `Ability2` are the free buttons PR #205 uses for roll and fly. |
| `InputManager` | `astraweave-input/src/manager.rs` | REAL and tested. Gameplay crates do not `use` it. The trace marks the crate example-only. |
| Bindings | `astraweave-input/src/bindings.rs` `BindingSet.actions: HashMap` | REAL. Iteration order is not a sim hazard today because every matched action receives the same pressed bit. |
| `InputContext` | stored on the manager | STUBBED. Queries do not filter by context. |
| Editor `GameInput` | `tools/aw_editor/src/runtime.rs` 429; filled from `viewport/widget.rs`; `inject_input` at `main.rs` 6120 | DORMANT. The field is stored. `game_input()` has no caller. Play mode does not move a capsule from it. |
| Editor bindings panel | `tools/aw_editor/src/panels/input_bindings_panel.rs` | REAL UI and a second binding model. It does not depend on `astraweave-input`. |
| Net `InputFrame` | `net/aw-net-proto` | REAL transport. The payload is an opaque blob, not these actions. UNTESTED against player locomotion. |
| Per-tick `InputFrame` in the game sim | — on `main` | ABSENT |

PR #205 is the input fix to reuse. It is open, not merged. Do not copy it onto this branch.

| PR #205 symbol | Role |
|---|---|
| `InputManager::set_physical_key` | `astraweave-input/src/manager.rs` on that branch. Window events, replay, and tests share one binding lookup. On `main` the lookup is still inlined in `process_window_event`. |
| `PlayInput::on_device_event` | `aravath_runtime/src/play_input.rs`. Look is `DeviceEvent::MouseMotion` while captured and focused. `CursorMoved` is not look. |
| `PlayInput::grab_attempts` | Windows tries only `CursorGrabMode::Confined`. Other OS values try `Locked`, then `Confined`. |
| `PlayInput::sample_tick` | Builds one `TickInput` from `Action` bits. Axes are `-1/0/1`. Roll is `Ability1`. Fly toggle is `Ability2`. Walk is Alt. |
| `FrameLatch` | `aravath_runtime/src/sim.rs`. Jump, roll, and attack edges are OR-held across frames that skip a 60 Hz slice. |

The sim on that branch never reads winit. That split is the contract.

### 2.4 Camera

| System | Path | Status |
|---|---|---|
| `CameraController` / free-fly | `astraweave-camera/src/freefly.rs` | REAL. Moves the camera, not a capsule. `process_mouse_delta` plus `raw_used_this_frame` already avoids applying absolute cursor and raw motion twice. |
| Editor orbit | `tools/aw_editor/src/viewport/camera.rs` | REAL editor camera. |
| Third person, occlusion shorten | `astraweave-camera/src/third_person.rs` on PR #205 only | ABSENT on `main` |
| Climb camera | — | ABSENT |

### 2.5 Animation, skinning, IK

| System | Path | Status |
|---|---|---|
| Clip sample, hierarchy, CPU skin | `astraweave-render/src/animation.rs` (`AnimationClip::sample` at 101) | REAL |
| GPU skin | `astraweave-render/src/skinning_gpu.rs` | REAL, feature-gated |
| glTF skeleton load | `astraweave-asset` | REAL |
| ECS `update_animations` | `astraweave-scene/src/lib.rs` ~896 | REAL time advance |
| `compute_poses_stub` | `astraweave-scene/src/lib.rs` 941–975 | STUBBED. Resizes matrices and sets `dirty`. Does not sample a clip. |
| Root motion | — | ABSENT |
| Motion matching | — | ABSENT |
| Foot IK, hand IK, two-bone IK | Editor toggles in `tools/aw_editor/src/panels/animation_panel.rs` | DORMANT UI. No solver. |
| Blend graph / state machine UI | same panel | DORMANT. The editor bridge ignores transition triggers. |
| `EditorAnimationBridge` | `tools/aw_editor/src/animation_bridge.rs` | REAL second sampler, using its own glTF types. |
| Editor `movement_scripts` | `tools/aw_editor/src/movement_scripts.rs` | REAL transform motion for scripted entities. Not a skeleton and not the physics capsule. |
| PR #205 `LocomotionGraph` | `aravath_runtime/src/locomotion.rs` | REAL on that branch. Samples `astraweave_render::animation` clips. States: Idle, Walk, Run, Sprint, CrouchIdle, CrouchMove, Jump, Roll, Fly, Attack, Block, Hit, Death. Displacement stays on the motor. |

Displacement on every live path is code-driven. Animation does not write the root.

### 2.6 PR #205 motor (not on `main`)

`aravath_runtime/src/motor.rs` is the only place sprint, crouch, roll, stamina, and a fly toggle actually change a capsule. It calls `control_character` and `jump`.

| Piece | Behavior | Gap against the brief |
|---|---|---|
| `MotorMode` | `Locomotion`, `Roll`, `Fly` | No climb, swim, glide, slide, prone, vault, mantle, shimmy, wallrun |
| Stamina | `f32`, sprint drains 18/s, regen 22/s on the ground, roll costs 25 and blocks regen during the roll | Regen is not defined for climb or glide because those states do not exist. A second pool, `Stats.stamina`, still exists and is unused by this motor. |
| Roll i-frames | `roll_time` in seconds, active from 0.08 to 0.32 of a 0.50 s roll | Not a tick window |
| Crouch | Speed 1.8. Capsule height unchanged. | No ceiling probe, no prone |
| Fly | Gravity scale 0, vertical speed from jump and crouch | A noclip toggle, not a sink-rate glide |
| `dt` | Argument to `tick` | The Aravath sim passes a fixed 1/60 slice. The motor itself does not enforce that. |

The motor's own comment says stamina lives here, not on `Stats`. Keep that. Do not add a third stamina field.

### 2.7 Assets already pinned

`assets/packs.manifest.toml` hashes packs by sha256. Relevant rows:

| Pack | Root | sha256 |
|---|---|---|
| `kaykit-complete` | `assets/The Complete KayKit Collection v4/` | `f63c732b14787377335a3092de6fde43bf96c2241f66907ccdbe5f027c862834` |
| `kenney-3d` | `assets/3D assets/` | `afebe5a16635d3fcf2fbbd03a75fb7780663767f4e3fa38f25b5768b035355c1` |

PR #205 adds a separate CC0 set under `assets/aravath/` with `assets/aravath/CREDITS.md`: KayKit Forest Nature Pack props, Kenney Nature Kit and Prototype Kit, Poly Haven textures, and the Quaternius Universal Animation Library (CC0) as `AnimationLibrary_Godot_Standard.gltf`. Clip names in `clip_name` match that library (`Idle_Loop`, `Jog_Fwd_Loop`, `Sprint_Loop`, `Crouch_Fwd_Loop`, `Roll`, `Jump_Loop`).

KayKit v7 is not in the manifest. It stays a candidate. The first animation pass reuses the Quaternius rig already chosen on PR #205. Any new pack is added to the manifest with a sha256 before it is referenced.

## 3. Determinism

### 3.1 What is canonical on `main`

| Requirement in the brief | On `main` at `85d09a2c` |
|---|---|
| `SimRng` | REAL. `astraweave-core/src/sim.rs` 18–81. `mix_sim_seed(world_seed, tick, system_id)` then `StdRng`. `unit_f32` is a dyadic rational. Weaving carries a duplicate locked to the same mix constant. |
| `Fixed64` | ABSENT. No `Fixed64`, `fixed64`, or fixed-point sim type in Rust sources or docs. |
| Fixed tick | REAL for the ECS schedule and for the NPC bridge. Not enforced inside `control_character`. |
| Replay gate | REAL. `.github/workflows/determinism-replay.yml` requires `planner_pure` and `perception_world` to match on Ubuntu, Windows, and macOS. |
| `Instant::now` in sim crates | Not in `astraweave-core`, `astraweave-ecs`, or `astraweave-gameplay` sources. Physics uses `Instant` only behind `async-physics` telemetry (`lib.rs` 1061). `examples/physics_demo3d` uses it for frame `dt`. |
| `thread_rng` in the move path | None found. Issue #225 tracks `thread_rng` in `llm_dialogue.rs`, which does not compile. That file is not a locomotion path. |

The current replay contract encodes `f32` with `to_bits` (`astraweave-core/tests/replay_support/mod.rs`). `replay_hash.rs` 23–26 writes `target/replay-hash.json` as `{ "case", "sha256" }`. The compare job's required set is line 95 of the workflow.

PR #205's determinism layer, also not on `main`, is `astraweave-physics/src/detmath.rs`: `libm::sqrtf` and `libm::acosf` for the jump impulse and the slope angle. That branch builds Rapier with `enhanced-determinism`, routes sim `sin`/`cos`/`atan2`/`sqrt`/`pow`/`exp` through `libm`, and refuses `target-cpu=native` and FMA. Its golden file is `aravath_runtime/golden/replay.hash`, checked by `.github/workflows/aravath-determinism.yml` on Linux and Windows. That workflow does not cover macOS, and it is separate from `determinism-replay.yml`.

`control_character` on `main` still calls `f32::sqrt` and `f32::acos` (jump at 1254, slope at 1355). Those are the two operations PR #205 wraps. A traversal sim that uses the unwrapped calls will not be cross-OS stable.

### 3.2 Hazards the traversal work has to close

| Hazard | Where | What the implementation does |
|---|---|---|
| Variable `dt` | `physics_demo3d` | The sim API takes a tick count. `dt` is the constant `1/60`. Frame time only chooses how many ticks to run. |
| `f32` slope and jump | `control_character`, `jump` | Call the PR #205 `detmath` wrappers. Do not add a third math module. |
| `Fixed64` missing | whole tree | See the review decision in §5. This plan does not invent `Fixed64` beside `detmath`. |
| Hash iteration | `buoyancy_bodies`, `char_map`, binding maps | Traversal scans sort by `BodyId` before applying a result that can change position. |
| Probe ties | none yet | Nearest hit, then lowest `BodyId`. The probe set is a fixed list, not a growable search. |
| I-frames in seconds | combat `IFrame`; PR #205 roll window | Tick integers. At 60 Hz the current roll window is ticks 5 through 19 inclusive of a 30-tick roll, and that conversion is data, not a float compare. |
| RNG | new slip or variant rolls | `SimRng::new(world_seed, tick, SIM_SYSTEM_TRAVERSAL)` with a new constant next to the existing `SIM_SYSTEM_*` values. Reseed every tick. |
| `CharState` stuck on `Grounded` | `control_character` | The new state component is the authority. `CharState` is not extended in parallel. |

### 3.3 Replay case (specified, not added)

This commit does not add the case. There is no traversal sim to hash, and a constant digest would make the gate vacuous.

When the first scripted ticks exist, the case is:

1. `traversal_bytes()` in `astraweave-core/tests/replay_support/mod.rs`, next to `perception_world_bytes`. Header rules stay: little-endian integers, length-prefixed UTF-8, `f32::to_bits`, sorted entity ids.
2. The script is 180 ticks at `dt = 1/60`, world seed `0x7A4E_0001`. Input is a fixed `TickInput` table, not wall-clock keys. The bytes include tick, state discriminant, position bits, vertical velocity bits, stamina bits, and the i-frame tick counter.
3. `astraweave-core/tests/replay_hash.rs` asserts two in-process runs match and appends `("traversal", sha256)`.
4. `.github/workflows/determinism-replay.yml` adds `"traversal"` to `required` so a mismatch fails the job on Ubuntu, Windows, and macOS.

The first implementation phase hashes ground, jump, and roll only. Later phases extend the same function. The case name stays `traversal`.

## 4. Upgrade plan

### 4.1 Where the code lives

One motor. After PR #205 merges, extend `aravath_runtime`'s motor only as the demo adapter. The sim types move into the engine so NPCs and the demo share them:

| Concern | Home | Why |
|---|---|---|
| Probes, kinematic move, surface flags on colliders | `astraweave-physics`, beside `control_character` | The capsule and the query pipeline already live here. A second controller is the failure this survey is trying to avoid. |
| State, stamina, tick windows | new module `astraweave-physics/src/traversal.rs` | Same crate as the body it moves. Plain data, serializable, no winit. |
| Water depth and oxygen | call `WaterQuery` and the existing `WaterPlayerState` | The swim module is dormant, not missing. Wire it. Do not rewrite the thresholds. |
| Input sampling | PR #205 `PlayInput` | Already the deterministic edge. New actions are new `Action` variants consumed by `sample_tick`. |
| Presentation | PR #205 `LocomotionGraph` plus `astraweave-render` clips | One sampler. The editor bridge stays an editor path. |
| Camera | PR #205 `ThirdPersonCamera` | Climb and glide change pitch limits and a shoulder offset. They do not grow a second camera type. |

`astraweave-core`'s grid `MoveTo` stays the tactical AI teleport. It is not the 3D motor. Editor `movement_scripts` stay scripted props.

### 4.2 State machine

One component, `TraversalState`, replaced only by a transition function. Guards read probes, stamina, and the tick clock. Enter and exit are explicit. Timers are `u16` tick counts.

`Ground`, `Air`, `Crouch`, `Prone`, `Slide`, `Roll`, `Vault`, `Mantle`, `LedgeHang`, `Shimmy`, `Climb`, `WallRun`, `Swim`, `Dive`, `Glide`.

`Fly` from PR #205 stays a separate debug mode with gravity off. It is not `Glide`. `Glide` has a capped sink rate, yaw steering, and a stamina drain.

Coyote time is 6 ticks. Jump and traversal-action buffering is 8 ticks. Both replace the `0.10` / `0.15` second fields when the motor is the caller. The old fields remain for current tests until those tests move to ticks.

Transitions that must be guarded:

| From | To | Guard |
|---|---|---|
| Ground, Air | Climb | Mode A or B accepts the surface, stamina above the attach cost, not in `Roll` |
| Climb | Climb | Overhang angle within the configured limit, stamina remaining |
| Climb | LedgeHang | Top probe hits a lip and the chest probe is blocked |
| LedgeHang | Shimmy | Lateral input, lip continues, clearance holds |
| LedgeHang, Climb | Mantle | Clearance capsule at the lip is empty and the height band matches |
| Ground, Air | Vault | Chest-high obstacle, empty landing, height band below mantle |
| Ground | Slide | Crouch held while sprinting, or slope above the slide angle |
| Slide | Air | Jump, momentum kept |
| Ground | Roll | Stamina at least the roll cost, not prone |
| Roll | Ground | Tick count reaches the recovery end. I-frames are a sub-range of those ticks. |
| Crouch | Ground | Ceiling clearance cast is empty |
| Crouch | Prone | Input, and the prone capsule fits |
| Prone | Crouch | Clearance for the crouch capsule |
| any grounded | Swim | Water depth crosses the swim threshold plus hysteresis |
| Swim | Dive | Dive held, or the head sample is under the surface |
| Dive | Swim | Head clears, breath state unchanged except the drain stops |
| Air | Glide | Glide held, falling, stamina above zero |
| Glide | Air | Stamina empty or glide released |
| Air | WallRun | Side probe hits a wall, speed above the minimum, wall flag allows it |
| WallRun | WallRun | A second wall within the chain window. Speed carries. |

Landing on a slope above the slide angle enters `Slide` instead of `Ground`, which is the roll-on-landing momentum rule for slopes. A landing roll is a separate transition from `Air` when the player holds roll inside the buffer.

### 4.3 Probes

Every probe is a Rapier query with the character body excluded. Results that tie on distance keep the lower `BodyId`. The set does not grow with scene size; each tick casts a constant number of rays and one clearance shape.

| Probe | When | Use |
|---|---|---|
| Chest forward | Air, Ground near a face, Climb | Wall contact, vault face |
| Down from above-forward | Chest hit, or climb near a lip | Ledge top, height band |
| Clearance capsule | Before mantle, vault, stand, or uncrouch | Reject the move when the destination overlaps |
| Hand and foot anchors (4) | Climb, Shimmy, LedgeHang | Average normal, detect overhang, feed hand IK targets |
| Water sample at feet and head | Always, via `WaterQuery` | Submersion for `WaterPlayerState` |
| Side capsule | Sprint or Air | Wallrun attach |

Height bands, measured from the feet to the ledge top: below `max_step` is a step (once step-up exists), then vault, then mantle, then climb or hang. The numbers are data on the controller, in metres, converted with the same `detmath` path as the rest of the sim.

`max_step` becomes a real step-up: a short upward move, a forward move, a downward snap, and a rejection when the riser exceeds `max_step` or the slope limit. The current "longer down ray" behavior is the bug this replaces.

### 4.4 Climbing

Three modes, selected per level, not compiled as three controllers. The probe and the stamina rules are shared. The attach rule is the mode.

**(A) Climb any surface.** Attach when the chest probe hits a face steeper than the walkable slope and the face's flag allows it. Each climb tick, the four anchors raycast along the previous normal. The capsule translation follows the averaged normal. Input moves in the tangent plane. Overhangs are in range while the normal's angle from world up stays under a configured limit (Conan keeps the underside; BotW drops it). Past that limit the state exits to `Air`. A climb jump spends a large stamina chunk and adds a tangent impulse. Rain or a wet material swaps the flag to slippery and applies a downward tangent bias.

**(B) Authored markup.** A `ClimbMarkup` component on an entity: an ordered list of handholds in local space, plus a link to the next hold. The solver picks the hold in the input direction inside a reach radius. The sim stores the chosen hold id. Animation warps to that point. This is the Uncharted / Assassin's Creed / Horizon path, and it is the readable path for puzzles that should look authored.

**(C) Hybrid. Recommended.** Try (B) when a markup hold is inside the reach radius. Otherwise try (A) if the flag allows it. The player can climb the mountain and still snap to a carved handhold when one is there. Markup wins over a nearby raw face so a puzzle ledge does not fight the surface probe.

Review can reject (C) and lock (A) or (B). The state enum does not change. Only the attach function does.

**Flags.** Stored on the collider, not inferred from the mesh name.

| Flag | Rule |
|---|---|
| `Climbable` | Mode A may attach. Default for static world geometry. |
| `Slippery` | Attach is allowed and a downward slide is added. Wetness and rain set this. Stamina drains faster. |
| `NonClimbable` | Attach fails. Default for anything spawned as a player-built piece. Puzzle volumes force this on the faces they contain. |

Fail closed: a player-built piece with a missing flag is `NonClimbable`. A static terrain collider with a missing flag is `Climbable`, matching the Conan / BotW default. An unknown dynamic body is `NonClimbable`.

**Stamina while climbing.** The climb state owns the drain. Regen is locked for the whole climb, including the frames where the player holds still on the wall. That is the Conan bug, and the test for it is mandatory. Modifiers, all multipliers on the drain:

- armor weight (a scalar on the motor; gear can write it later)
- a climb-gear scalar (boots / gloves), default 1
- wetness from `WaterPlayerState::wet_status`
- `Slippery` flag

Empty stamina forces `Air` with the current velocity. There is no regen until `Ground` or `Swim` (swim has its own rule).

**Collision fidelity.** A climbable collider must stay within a budget of its render mesh. The lint measures the maximum distance from render-mesh samples to the collider surface and fails the asset when that distance exceeds the budget (start at 0.05 m). Hull-only cliffs are `NonClimbable` until the hull is tight enough, because a loose hull is the floating-hands failure Funcom hit. The lint is an offline check in the asset validator, reading the manifest path, and it runs on the showcase cliff before the demo lists that mesh as climbable.

**Exploit tests**, each a sim replay, not a manual note:

- Regen during climb stays zero, including a zero-input hang.
- A `NonClimbable` building piece cannot be attached, including from a jump and from a markup hold that was moved onto it.
- A puzzle volume cannot be crossed by climbing around its forced flag.
- Overhang past the angle limit detaches. It does not launch the body upward.
- Slippery rain cannot be cancelled by toggling crouch or roll.
- Two players on one face resolve probes in `BodyId` order and produce the same bytes on all three OS runners.

### 4.5 The other moves

| Move | Sim rule | Animation |
|---|---|---|
| Swim | `WaterQuery` depth at the capsule. Hysteresis of 2 ticks around the existing `WaterPlayerState` thresholds so a wave sample cannot flicker the mode. Horizontal move uses the swim speed multiplier. | Surface stroke. Code moves the root. |
| Dive | Full 3-axis wish, damped each tick. Oxygen is the existing `WaterPlayerState` breath meter. Empty oxygen applies the existing drowning damage. | Dive clip. |
| Glide | Constant sink, yaw rate from look, stamina drain, regen locked. Updraft volumes add a fixed lift while the body is inside an AABB. | Arms-out pose. No root motion. |
| Prone / crawl | Second, shorter capsule. Pitch follows the ground normal. Turn rate is capped in yaw per tick. Stand and crouch require a clearance cast. | Prone clip when the Quaternius set has one; otherwise a lowered crouch clip, labeled as such in credits. |
| Slide | Enter from sprint-crouch or a steep landing. Speed carries and decays by a friction constant per tick. Downhill adds a fixed acceleration. Jump cancels into `Air` and keeps horizontal speed. | Slide clip, phase from the tick counter. |
| Roll / dodge | 30 ticks. I-frames on ticks 5 through 19. Costs stamina. No regen during the roll. Direction is the wish, or the facing when the wish is zero. The combat sweep reads the motor's tick window. `IFrame.time_left` stops being the authority for the player. | Existing `Roll` clip. `visual_time` maps the tick phase onto the clip. |
| Vault | Speed-preserving arc over a chest-high obstacle when the clearance capsule is empty. Duration is a tick count. | Warp to the sim's landing point. |
| Mantle | Slower than vault, ends in `Ground` or `Crouch` on the lip. | Hands use the lip point as an IK target once IK exists. Until then, the root still lands on the sim point. |
| Ledge hang / shimmy | Hang holds the lip point. Shimmy moves along the lip in the camera's lateral axis and detaches at a gap larger than the reach. | Hand IK targets are the lip samples. |
| Wallrun | Side probe, minimum horizontal speed, a per-tick speed floor, and a tick limit. A chained wallrun inside the window keeps speed. Gravity along the wall is reduced by a constant. | Wall pose. Camera yaws to the wall normal on the presentation side only. |
| Sprint | Speed tier above run, drains stamina, stops draining and drops to run at zero. Regen on `Ground` while not sprinting, not climbing, not gliding, not rolling. | Existing `Sprint_Loop`. |

Momentum that must survive a chain: slide into jump, wallrun into wallrun, roll on a hard landing. Each chain is one replay assertion.

### 4.6 Stamina model

One `f32` meter on the motor, the one PR #205 already has. `Stats.stamina` remains the combat snapshot and is not a second pool. A later combat change can copy the motor value into `Stats` at snapshot time. This work does not do that copy.

| State | Drain | Regen |
|---|---|---|
| Sprint | `SPRINT_DRAIN` per tick | off |
| Roll | one-shot `ROLL_COST` | off until the roll ends |
| Climb, Glide | per-tick, times weight, gear, wetness, slippery | off, including while idle on the wall |
| Swim sprint | per-tick | off while sprinting |
| Dive | oxygen, not stamina, using `WaterPlayerState` | oxygen recovers on the surface |
| Ground, idle, walk, run | none | `STAMINA_REGEN` per tick, capped |

Rates move from "per second" constants to "per tick" constants at 60 Hz so a hitch cannot change the drain. The numeric result at 60 Hz matches today's 18/s and 22/s.

### 4.7 Animation and IK

The sim owns displacement. The graph reads `TraversalState` and never writes a position back.

1. Extend `LocomotionGraph::choose_state` and `clip_name` with the new states. Missing CC0 clips use the nearest existing clip and a credit note, rather than a second rig.
2. Blend by a fixed tick crossfade (4 ticks) inside the graph. That is the blend graph. It is presentation-only.
3. Foot IK: a downward ground probe from each foot bone, a pelvis offset, predictive by one tick from the sim velocity. This is presentation. It does not move the capsule.
4. Hand IK: the climb and shimmy probes already produce the four anchor points. Hands solve toward those points. Two-bone, no solver crate until a CC0-compatible dependency is reviewed. A small in-crate two-bone function is enough and stays out of the sim hash.
5. Motion matching is a later option: a pose-and-trajectory database, searched on the presentation side, never feeding the sim. It is out of scope until the state graph is in the demo.

Root motion stays unused. If a clip later drives a warp, the warp target is the sim's chosen point and the delta is a baked per-tick table, not a runtime curve evaluated with platform `sin`.

### 4.8 Demo

The showcase is the Aravath greybox, in the demo PR #205 already runs, after that PR has merged. Player-facing text says Aravath. This document does not rename the older slice example.

One course, in order, so a replay script can walk it:

1. Sprint gate that empties stamina, then a regen beat on flat ground.
2. A roll lane with a hazard that overlaps ticks 5–19 and a second hazard that overlaps the recovery.
3. A chest-high vault and a taller mantle.
4. A crawl gap that rejects standing.
5. A slope that enters the slide and a jump that keeps the speed.
6. A wallrun alley of two walls.
7. A raw cliff (mode A), an overhang, and a wet streak that slips.
8. A marked handhold face (mode B) that the hybrid prefers over the raw rock beside it.
9. A puzzle wall flagged `NonClimbable`, and a player-built piece that fails closed.
10. A pool: wade, swim, dive, breath empty.
11. A ridge and a glide across an updraft to the far ledge, then a shimmy to the exit.

The course geometry is built from the CC0 packs already credited. The cliff used for mode A has to pass the collision-fidelity lint.

### 4.9 Tests and benches

| Layer | What |
|---|---|
| Transition guards | One test per row in §4.2, including the rejected cases (stand under a ceiling, climb a `NonClimbable` piece, roll at zero stamina, regen during climb). |
| Probes | Fixed scenes: step, vault, mantle, overhang, gap. Assert the chosen `BodyId` and the height band. A tie fixture with two colliders at the same distance asserts the lower id wins. |
| Replay | §3.3, required on three OS runners. |
| Exploits | The list in §4.4. |
| Water | The dormant `WaterPlayerState` tests stay. New tests feed `WaterQuery` samples into that state and check hysteresis. |
| Lint | One fixture mesh inside the budget and one hull outside it. |
| Bench | A new `traversal_probes` bench: one character and 64 characters, fixed tick, budget recorded in the bench name. `character_full_tick` is fixed by resetting the body each iteration, or by documenting that the finite ground is 100 m wide and the loop walks off it. The `y > -10` assertion stays. Issue #221 already tracks the red bench shards; the reset is the traversal-owned part of that failure. |

Perf budgets start as measurements, not guesses. The first recorded number becomes the budget in the bench file.

### 4.10 Phases after approval

Each phase is reviewable on its own and keeps the replay case green.

| Phase | Lands | Does not land |
|---|---|---|
| 0 | Replay case for ground, jump, and tick-window roll. `detmath` acos/sqrt on the existing controller. Tick constants. Bench reset for `character_full_tick`. | New moves |
| 1 | `TraversalState` for Ground, Air, Crouch, Sprint, Roll, wired through the Aravath motor. Stamina per tick. Capsule unchanged except crouch speed. | Climb |
| 2 | Probe set, real `max_step`, vault, mantle. | Climb flags |
| 3 | Climb mode C, flags, overhang, wetness, exploit tests, collision lint. | Swim |
| 4 | Wire `WaterPlayerState` to `WaterQuery`. Swim and dive. | Glide |
| 5 | Slide, prone, wallrun, glide, ledge hang, shimmy. Showcase course. | IK |
| 6 | Foot and hand IK on the presentation graph. New CC0 clips only if pinned. | Motion matching |
| 7 | Motion matching, only if phase 6 is in the demo and a review asks for it. | — |

Phase 0 is the first commit that touches `determinism-replay.yml`.

## 5. Decisions for review

1. **Climb mode.** This plan recommends (C), the hybrid. (A) and (B) stay selectable per level because the attach rule is data.
2. **Numeric type.** `Fixed64` is not on `main`. PR #205's contract is fixed 60 Hz, `f32::to_bits`, `libm` via `detmath`, and Rapier `enhanced-determinism`. This plan follows that contract and does not add a fixed-point type. Say so if the review wants `Fixed64` introduced before any traversal state.
3. **PR #205.** Implementation starts from `main` after that PR merges, or on top of its branch if the review says to stack. This branch does not reimplement `set_physical_key`, `PlayInput`, `grab_attempts`, `FrameLatch`, `ThirdPersonCamera`, `LocomotionGraph`, or `detmath`.
4. **Scope of phase 0.** Confirm that resetting `character_full_tick` is in phase 0, with issue #221 left open for the unrelated bench timeouts.

## 6. Pre-existing failures

Checked `main` at `85d09a2c` on 2026-10-15. `determinism-replay` succeeded (run 38617502511). These jobs were already red and already have write-ups:

| Failure | Run | Issue | Traversal action |
|---|---|---|---|
| `Build & Test (ubuntu)` terrain golden tests | 38617502405 | #242. Seven `astraweave-terrain` golden panics. Plan in the issue is to recompute or fix the drift, not to weaken the asserts. | Out of scope. Not a locomotion path. |
| Performance Benchmark shards, including `astraweave-physics` `character_full_tick` | 38617502113 | #221, labeled `deferred-failure`. Also #224. | The walk-off assertion is in scope for phase 0. The 600 s shard cap is not. |
| Copilot code review on the same SHA | 38617507299 | No engine test failure attached. | No code change. |

#242's title says deferred-failure and its body has a plan. The GitHub label is not applied, and the issue names no owner. The same label gap shows on #226–#241. This branch does not edit issues. The terrain golden drift stays on #242.

Issue #225 (`llm_dialogue.rs`, `thread_rng`) is labeled and is not a movement system.

## 7. Trace notes for the first code commit

`docs/architecture/physics.md` is at version 1.2, verified 2026-05-12. It disagrees with this tree:

| Trace | Code at `85d09a2c` |
|---|---|
| `add_water_aabb` is an empty stub | It calls `self.water.add_aabb` (1493–1495). |
| `control_character` cited near line 1247, and `set_translation` in the pipeline notes | The function is at 1258 and commits with `set_next_kinematic_position`. |
| Combat sweep takes an `iframe_map` | `IFrame` lives on `Combatant`. Nothing decrements `time_left`. |
| Invariant that every caller passes `dt = 1/60` | `examples/physics_demo3d` passes a frame `dt` from `Instant::now`. |

This commit does not edit the trace, because it does not edit the crate. The phase 0 commit updates §5, §8, and §11 of `physics.md`, plus the input, animation, and water traces for whatever those phases touch.

## 8. Stop

No feature code, no replay-case edit, and no merge until the four decisions in §5 are answered.
