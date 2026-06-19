# AstraWeave Project Status

> **Last Updated**: June 10, 2026  
> **Read by**: Copilot agent when it needs current project context  
> **Do not inline this into copilot-instructions.md** — point to it instead

---

## Active Work

### Editor Multi-Tool Architecture Campaign — Sub-phase 5 IN PROGRESS (Jun 6, 2026)
- **Sub-phase 3** (Mediator Brush Architecture) COMPLETE 2026-05-14 (`b220442a7`); **Sub-phase 4** (dispatcher Pattern A regression infrastructure) COMPLETE 2026-06-06 (`d5c350f60`)
- **Sub-phase 5** (RegionalArchetypePanel ActiveTool): 5.A dispatcher Y=0 projection path (`85786bf70`) and 5.B panel ActiveTool + registration + paint command/undo (`3cdb23239`) landed 2026-06-06
- **Remaining**: Sub-phase 5.C Andrew-gated closeout, Mediator Removal session, Sub-phase 6 campaign closeout
- **Plan**: `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` (note: its §11 Sub-phase 5 entry predates the 5.A/5.B commits — git evidence is authoritative)

### Campaign Hand-offs (open follow-up queues)
- **Terrain Asset Quality** (closed 2026-06-02): cleanup queue + pause stack in `docs/audits/terrain_asset_quality_outcome_2026-06.md` §5/§7 — notably the broken BC7/KTX2 cook path (placeholder encoder, DFD sRGB bug; runtime uploads uncompressed RGBA8) and the polyhaven showcase pack's 10 missing files behind a warn-logged fallback
- **Unified Camera** (closed 2026-06-01): cleanup queue in `docs/audits/unified_camera_outcome_2026-06.md` §5
- **Net-Trio-Remediation** (closed 2026-06-10): deliberate boundaries deferred to future session-security work — no replay/nonce protection; server→client unsigned (asymmetric-trust design)

### Phase 8.8: Physics Robustness Upgrade — PAUSED (no commits since Feb 2026)
- **Objective**: Bring all physics subsystems to fluids-level quality
- **Baseline**: Fluids system A+ grade with 2,404 tests (benchmark caliber)
- **Current**: ~500 physics tests → 657+ target (157 new tests planned)
- **Priority 1**: Spatial Hash (C → A), Async Scheduler (D+ → B+), Projectile (C+ → A-)
- **Timeline**: 4 phases, ~30 hours total
- **Plan**: `docs/current/PHASE_8_8_PHYSICS_ROBUSTNESS.md`

**Phase 1 (Priority 1)**: Critical gaps (8-10h, 77 tests)
- Spatial Hash: +27 tests (stress, edge cases, cell boundaries)
- Async Scheduler: +21 tests + TODO fix (line 154 parallel pipeline)
- Projectile: +29 tests (ballistics, penetration, explosions)

**Phase 2 (Priority 2)**: Coverage gaps (10-12h, 80 tests)
- Destruction: +23 tests (chain reactions, stress propagation)
- Cloth: +20 tests (tearing, wind interaction, self-collision)
- Ragdoll: +17 tests (joint limits, pose blending, fall recovery)
- Vehicle: +10 tests (Pacejka tire model, suspension)
- Gravity: +10 tests (inverse-square law, orbital mechanics)

**Current Subsystem Grades**:

| Subsystem | Grade | Tests | Key Gap |
|-----------|-------|-------|---------|
| Fluids | A+ | 2,404 | Benchmark (complete) |
| Core/CharacterController | A | 110+ | NaN/Inf coverage done |
| Environment | A- | 55+ | Wind/buoyancy done |
| Vehicle | B+ | 50+ | Missing Pacejka validation |
| Gravity | B+ | 30+ | Missing inverse-square validation |
| Cloth | B | 25+ | Missing stress tests |
| Ragdoll | B | 33+ | Missing joint limit tests |
| Destruction | C+ | 17 | Missing chain reaction tests |
| Projectile | C+ | 21 | Missing ballistics validation |
| Spatial Hash | C | 8 | Critical for O(n²) optimization |
| Async Scheduler | D+ | 4 | Incomplete parallel pipeline |

### Phase 8: Game Engine Readiness — Overall Progress
- **Mission**: Transform from "production-ready infrastructure" to "ship a game on it"
- **Started**: October 14, 2025
- **Current Gap**: 60-70% complete for shipping full games
- **Note (Jun 2026)**: the priority-track day counters below have been dormant since early 2026 — May–June work went to the editor/camera/parity/net campaigns listed under Recently Completed

**Priority Tracks**:
1. **In-Game UI Framework** (5 weeks) — 72% complete (18/25 days, 3,573 LOC)
   - Weeks 1-3 COMPLETE. Week 4 Day 3 last completed.
   - Next: Week 4 Day 4 (Minimap improvements)
2. **Complete Rendering Pipeline** (4-5 weeks) — Shadow maps/post-FX infrastructure exists
3. **Save/Load System** (2-3 weeks) — Deterministic ECS ready
4. **Production Audio** (2-3 weeks) — Mixer/crossfade already exist

### Future Planning
- **Phase 9.2: Scripting Runtime Integration** (6-9 weeks)
  - Sandboxed Rhai scripting for modding
  - Plan: `docs/current/PHASE_9_2_SCRIPTING_INTEGRATION_PLAN.md`

---

## Recently Completed

### Net-Trio-Remediation W.1–W.5 ✅ (June 9–10, 2026)
- **Defect fixed**: the long-standing HMAC-vs-XOR `sign16` mismatch in the standalone matchmaking trio (`aw-net-{proto,client,server}`) — previously every signature verification failed and the server only warned
- **W.1**: canonical HMAC-SHA256 surface in `aw-net-proto` (`SigningKey` with private key bytes, constant-time `verify` via `Mac::verify_slice`, `input_frame_sig_payload`, `SIG_LEN=32`); XOR `sign16` stub + `SessionKey` deleted
- **W.2**: client signs via the canonical surface (`AW_SHARED_KEY`, fail-fast); server verifies FIRST (before any state mutation) on both TLS and plain handlers with `SignatureFailurePolicy { Kick (default), Warn }` — Kick issues WebSocket Close 1008 through the real disconnect/cleanup path
- **W.3/W.5**: 5 test families (authenticated round-trip, tampered/malformed reject-and-survive, wrong-key policy, disconnect paths, TLS signature path) + RFC 4231 KATs — trio went 53 → 104 tests, all green
- **Deliberate boundaries (not defects)**: no replay/nonce protection; server→client unsigned (asymmetric-trust design)
- **Docs**: `docs/audits/net_trio_signature_remediation_findings_2026-06.md`; `net/README.md` rewritten; `net_ecs.md` rev 1.3; ARCHITECTURE_MAP bumped v0.7.1
- Commits `561b20957` → `017ada12c`

### Editor Multi-Tool Sub-phase 4 ✅ + Sub-phase 5.A/5.B (June 6, 2026)
- **SP4 COMPLETE** (`d5c350f60`): Pattern A regression net for the dispatcher class — active_tool unit tests 15 → 21 (registration/dup-overwrite, deactivate-before-activate ORDER, dispatch routing, mutex enforcement, `EventDisposition` forward-compat tripwire)
- **SP5.A** (`85786bf70`): dispatcher Y=0 projection path lived in `viewport/widget.rs`; **SP5.B** (`3cdb23239`): `RegionalArchetypePanel` ActiveTool + dispatcher registration + paint command/undo
- SP5.C Andrew-gated closeout pending (campaign still open)

### Terrain Asset Quality Campaign ✅ (May 14 – June 2, 2026)
- **Verdict**: ACCEPTABLE at residency (80 MiB/active 5-layer pack = 31% of the 256 MB soft texture budget) + OPTIMIZATION-CANDIDATE on format
- 9 Tier-1 PBR material PNG triples acquired (PolyHaven + ambientCG; provider extended with ARM-map fetch); founding "replace 22 placeholders" premise falsified at A.3 — real PBR was already deployed
- **Durable findings**: BC7/KTX2 cook path broken (no toktx/basisu, placeholder encoder writes monochrome blocks, DFD sRGB bug) so runtime uploads uncompressed 1024² RGBA8; 27-PNG runtime-vs-source divergence; polyhaven showcase pack references 10 missing files behind a warn-logged fallback
- **Docs**: `docs/audits/terrain_asset_quality_outcome_2026-06.md` (closeout `61332f75c`)

### Unified Camera Campaign C.0–C.9 ✅ (May 18 – June 1, 2026)
- Consolidated 8 divergent camera codepaths (3 competing yaw conventions, dual renderer upload paths, 3 parallel cinematics keyframe systems)
- **New crate `astraweave-camera`** (`52b9e711c`): canonical `Projection`/`RenderView`/`CameraProducer` types; `Renderer::update_view` is the sole upload entry point; dual `update_camera`/`update_camera_matrices` paths deleted (50 files)
- OrbitCamera `fov`(deg) → `fovy`(rad) rename with serde back-compat; editor `CameraKeyframe` retired into cinematics `CameraKey`; gizmo `CameraController` + `SceneViewport` deleted (net −1,549 LoC in C.6)
- `FreeFly::sanitize()`/`CameraKey::sanitize()` hardening; `docs/current/CAMERA_CONVENTIONS.md` + 8 contract tests + parity-harness matrix fixtures (C.8)
- **Docs**: `docs/audits/unified_camera_outcome_2026-06.md` (closeout `017f65da7`)

### Editor-Engine Render Parity P.1–P.7 ✅ (May 17, 2026)
- Bit-identical per-machine parity (SHA-256 of LDR bytes) between editor viewport and standalone `Renderer::draw_into` — the WYSIWYG contract, publicly enforced by `tools/aw_editor/tests/render_parity_harness.rs`
- Five seams closed: shared loader, shared tonemap (editor `tonemap.wgsl` + `hdr_blit_pipeline` deleted, ~550 LoC; shared ACES `post_pipeline` is the single terminal stage), shared `GameQuality` preset, target-format equality, overlay isolation (`ENGINE_LDR_TARGET` + `EDITOR_OVERLAY_TARGET` + `composite.wgsl`)
- Editor's multi-operator tonemap authoring (PBR Neutral / AgX UI surface) intentionally removed per P.0 Q3 decision — engine operators are ACES (default), AgX, Reinhard
- **Docs**: `docs/audits/editor_engine_render_parity_outcome_2026-05.md` (closeout `6dc95ae9b`)

### Editor Multi-Tool Sub-phase 3 + Cleanup Chain ✅ (May 4–14, 2026)
- Mediator Brush Architecture complete: Real-Fix.A/B/C/D/E + Cleanup-A/B/D resolved four §7.7 wrapped-component resource-identity instances; 8/8 brush modes operational; erosion chunk seams eliminated (M-D5/M-D9/M-SK continuity fixes)
- Legacy cluster render path deleted; canonical MaterialLibrary now 21 named + 11 reserved (32-slot capacity preserved, `24b1b2150`)
- **Docs**: `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` (closeout `b220442a7`)

### ARCHITECTURE_MAP v0.7.0 Reconciliation + Interactive Workspace Map ✅ (May 13, 2026)
- ARCHITECTURE_MAP.md reconciled against the 13 pre-existing subsystem traces and bumped to v0.7.0 (`a0b254042`); interactive `docs/architecture/workspace_map.html` visualization deployed (`ee8e3f579`, `44e769c4b`)
- Map has since reached v0.7.2 (Net-Trio reconciliation 2026-06-10 + member-count/camera reconciliation)

### Allocation Measurement + mimalloc Merge ✅ (April 17, 2026)
- **Audit**: `docs/audits/allocation_audit_2026-04-17.md` — static survey confirmed no external allocator, arena, pool, or GPU sub-allocator crate anywhere; hot allocation sites catalogued with file:line citations.
- **Instrumentation**: `docs/audits/allocation_measurement_plan_2026-04-17.md` — added `alloc_plot!`/`measured_span!` macros + `FrameAllocStats` to `astraweave-profiling` (§1.3 of that plan), extended `CountingAlloc` in `astraweave-ecs` with `reallocs()`/`bytes_allocated()`/`reset()`, wired eight Tracy span+plot pairs into the four hot paths (`render.submit`, `render.visible_instances`, `render.bin_lights_cpu`, `physics.step`, `ai.tick`, `ai.goap.plan`, `ecs.schedule.run`, `ecs.schedule.build_groups`), added four `alloc_measure` criterion benches (ecs/physics/render/ai) with allocation-count assertions, and a non-blocking `.github/workflows/allocation-measurement.yml`. Every symbol is feature-gated; release builds without `alloc-counter` or `profiling` are zero-cost.
- **Experiment**: `docs/audits/mimalloc_experiment_2026-04-17.md` — paired baseline vs. mimalloc measurement, three independent runs per cell. Results: `ecs.schedule.run` −23%, `ai.goap.plan` **−52%**, `profiling_demo` FPS **+43%** (956 → 1369 median at 1000 entities). `physics.step` and `render.bin_lights_cpu` changed within noise. Allocs/bytes/reallocs/frame identical across allocators (sanity check passes).
- **Merged**: `fast-alloc` on by default in `examples/profiling_demo`, `examples/hello_companion`, `tools/aw_editor`. Library crates (`astraweave-ecs`, `-physics`, `-render`, `-ai`, `-profiling`) unchanged — they keep `fast-alloc` opt-in. Opt-out path `--no-default-features` preserved and verified (opt-out median 855 FPS vs default 1458 FPS; matches pre-merge ranges).
- **New crate**: `crates/astraweave-alloc` — tiny single-purpose crate. `MiMalloc` re-export behind `fast-alloc` feature + `setup_global_allocator!()` macro. MIT-licensed (mimalloc is Microsoft-maintained MIT, no extra build deps on Windows/Linux/macOS).
- **Incidental fix**: pre-existing `moved_count` typo in `examples/profiling_demo/src/main.rs:448` that blocked the `profiling,alloc-counter` compile-combo (latent since the profiling instrumentation landed). Minimal rename, preserved `#[allow(unused_variables)]` for the `profiling`-off case.

### Impostor Atlas Bake + LOD3 Sampling Pipeline ✅ (April 16, 2026)
- **Scope**: Task 2A-5 (tree LOD chain billboard/impostor) from `RENDERING_REMEDIATION_PLAN.md`. Closes the visual gap where distant scatter instances rendered as untextured "gray monolith" impostor cards.
- **Crate**: `astraweave-render` behind `impostor-bake` feature (off by default for lean builds).
- **T1 — ImpostorBaker** (`src/impostor_bake.rs`): GPU-side atlas renderer. Owns colour + depth targets, unlit-with-alpha bake shader (`IMPOSTOR_BAKE_WGSL`), per-cell viewport+scissor writes with `LoadOp::Load` so cells accumulate. Non-MSAA single layer `Rgba8UnormSrgb`.
- **T3 — Atlas I/O** (`src/impostor_bake.rs`): `save_atlas_png`/`load_atlas_png` (gated on `textures` feature) + TOML sidecar (`ImpostorAtlasSidecar`) with bidirectional `From` to `ImpostorAtlasSpec`. Lookup semantics preserved byte-for-byte across roundtrip.
- **T4 — LOD3 sampling pipeline** (`src/impostor_lod3.rs`): Billboard-facing vertex shader that picks an atlas cell from `atan2(forward.x, forward.z)`, alpha-test fragment (< 0.05 discard). `SpeciesRowGpu` (32 B) + `Lod3InstanceRaw` (32 B) + `Lod3Pipeline` + `build_lod3_pipeline` factory + `Lod3Resources::upload` helper that wraps atlas texture + sampler + species-rows storage buffer + pre-built atlas bind group.
- **T5 — `fit_ortho_camera`**: AABB-aware orthographic fitter. Uses `sqrt(sx² + sz²) / 2` half-width (not `max`) so all orbit angles frame the mesh cleanly; near/far planes bracket eye distance.
- **T6 — Lazy-bake orchestrator** (`load_or_bake_atlas`): sidecar schema match → load PNG, cache miss/corrupt/mismatch → callback rebake → persist both files. Callback pattern keeps lazy-bake path GPU-agnostic.
- **T8 — GPU integration tests**: `tests/impostor_bake_pipeline.rs` (8 tests, bake end-to-end + readback regression guards) + `tests/impostor_lod3_pipeline.rs` (5 tests, pipeline build + upload + row-order regression).
- **T2 — `aw-impostor-bake` CLI** (`src/bin/aw_impostor_bake.rs`): Offline CLI behind a dedicated `impostor-bake-cli` feature. `--input` (repeatable, one per species) → `load_gltf` → `SimplificationMesh::from_cpu_mesh` → `upload_simplification_mesh` → `make_diffuse_bind_group` (magenta fallback) → per-angle `fit_ortho_camera` + `draw_into_region` → `readback_atlas` → `save_atlas_png` + `save_atlas_sidecar`. `[[bin]] required-features = ["impostor-bake-cli"]` keeps `clap`+`pollster` out of default builds. Sidesteps the `astraweave-render → aw_asset_cli` circular dep that blocked a `tools/` location.
- **T7 stage 1 — `ImpostorPass` draw helper** (`src/impostor_pass.rs`): Reusable bundle of (pipeline + atlas resources + camera UBO + shared unit-quad vbuf/ibuf + auto-growing instance buffer). `new()` builds everything in one shot; `update_camera`/`upload_instances` are per-frame; `record(&mut RenderPass)` issues the indexed instanced draw. 4 unit tests (layout + quad geometry + CCW winding) + 4 GPU integration tests (including end-to-end render-and-readback of a red-atlas billboard into an offscreen Rgba8UnormSrgb target).
- **T7 stage 2 — `Renderer::install_impostor_pass`** (`src/renderer.rs`): Added `impostor_passes: HashMap<String, ImpostorPass>` field to `Renderer` (feature-gated on `impostor-bake`, rev 2 after multi-species discovery during stage 3 planning — was `Option<ImpostorPass>` in rev 1) + keyed public API `install_impostor_pass(key, pass)` / `remove_impostor_pass(key)` / `clear_impostor_passes()` / `impostor_pass_mut(key)` / `has_impostor_pass(key)` / `impostor_pass_count()` + `hdr_format()` / `depth_format()` accessors so callers can build passes with matching formats. Injection loops over the map values in both render paths — `render()` (swap-chain present, ~L5081 before `water.render`) and `draw_into()` (editor/offscreen path, ~L5641 before `water.render`) — recording only passes with `instance_count > 0`. Keys are caller-chosen (typically a `MeshHash` hex from `viewport::impostor_registry`). All 41 impostor-family lib tests + 4 GPU integration tests pass; editor compiles with and without the feature.
- **T7 stage 3a — `viewport::impostor_registry`** (`tools/aw_editor/src/viewport/impostor_registry.rs`, ~340 LOC): Editor-side plumbing for per-scatter-mesh atlas caching. `MeshHash` (SHA-256 of source bytes → 64-char lowercase hex) + `atlas_paths(cache_root, hash) → (<root>/<hash>/atlas.png, atlas.toml)` + `ImpostorRegistry::{new, ensure, get, contains, evict, len, is_empty}` that wraps `astraweave_render::impostor_bake::load_or_bake_atlas`. `ensure` is idempotent: in-memory hit → no-op; disk-cache hit → load + cache; miss → invoke bake callback, persist PNG+TOML, cache. 8 unit tests: hash determinism (SHA-256 spot-check), hex-form validator (length + alphabet), path layout, mem-cache idempotency, disk persistence, warm-registry disk reload, evict semantics (disk preserved). Gated behind a new editor-level `impostor-bake` feature that re-enables `astraweave-render/impostor-bake`; default build unchanged.
- **T7 stage 3c.1 — `viewport::impostor_wiring`** (`tools/aw_editor/src/viewport/impostor_wiring.rs`, ~280 LOC): Editor-side helpers that bridge scatter LOD3 primitives to the shared bake + pass infrastructure. `primitive_mesh_hash(CpuMesh) -> MeshHash` hashes vertex + index + albedo bytes (SHA-256) so two scatter groups sharing the same mesh collide on the same cached atlas. `bake_primitive_pixels(device, queue, mesh, spec, label) -> Result<(pixels, w, h)>` drives a single-species `ImpostorBaker`: fits an ortho camera per angle via `fit_ortho_camera(aabb)`, renders each cell via `draw_into_region`, and reads back the atlas — suitable both as a `load_or_bake_atlas` closure and as direct input to `ImpostorPass::new`. Private helpers convert `CpuMesh` → `SimplificationMesh` (positions/normals/UVs/indices) and upload a bake-time diffuse (1×1 opaque white fallback when the mesh has no albedo). 4 unit tests: determinism, content-addressed hashing (albedo change → different hash), None-vs-Some albedo discrimination, SimplificationMesh topology preservation. Gated on `impostor-bake`.
- **T7 stage 3c.2 — adapter registry field + retirement helpers** (`tools/aw_editor/src/viewport/engine_adapter.rs`): Added `impostor_registry: Option<ImpostorRegistry>` field (initialized in `EngineRenderAdapter::new` rooted at `assets/cache/impostors/`) and `installed_impostor_keys: HashSet<String>` field for tracking currently-installed `Renderer::install_impostor_pass` keys, both `#[cfg(feature = "impostor-bake")]`-gated. Added `default_impostor_cache_root()`, `retire_all_impostor_passes()`, `retire_stale_impostor_passes(live_keys)`, and `installed_impostor_pass_count()` (the latter has a `cfg(not(...))` companion returning 0, so instrumentation works in both configs). Wired `clear_scatter` to retire all impostor passes alongside the existing PBR model drain, mirroring `scatter_model_names` retirement semantics. 1 new unit test (cache-root path components) + 4 existing impostor_wiring tests still green; both default and `impostor-bake` configs compile clean.
- **T7 stage 3c.3-a — `upload_impostor_pass_for_primitive` helper** (`tools/aw_editor/src/viewport/engine_adapter.rs`): Self-contained method on `EngineRenderAdapter` (feature-gated on `impostor-bake`) that encapsulates the full per-primitive impostor upload flow — content-addressed hashing via `primitive_mesh_hash(full_mesh)`, lazy bake-or-load via `registry.ensure(hash, spec, bake_primitive_pixels)`, `ImpostorPass::new(..., renderer.hdr_format(), Some(renderer.depth_format()))`, `renderer.install_impostor_pass(key, pass)` registration, and per-frame `update_camera(queue, view_proj, camera_pos)` + `upload_instances(device, queue, &[Lod3InstanceRaw])`. Fast path checks `renderer.has_impostor_pass(key)` and skips the bake/install branch. Uses `Arc`-cloned `wgpu::Device`/`wgpu::Queue` handles to satisfy the borrow checker when `registry.ensure` (&mut self) and `renderer.impostor_pass_mut` (&mut self) both need access. Returns `Result<String>` with the install key so the caller can track it in a live-keys set for `retire_stale_impostor_passes`. All 13 impostor unit tests still green; both default and `impostor-bake` configs compile clean.
- **T7 stage 3c.3-b — LOD3 call-site swap** (`tools/aw_editor/src/viewport/engine_adapter.rs` + `astraweave-render/src/renderer.rs`): Swapped the scatter LOD3 rendering path from the legacy PBR-quad fallback to the new `ImpostorPass` pipeline under `#[cfg(feature = "impostor-bake")]`. The `ImpostorCard` match arm now populates a parallel `Vec<Lod3InstanceRaw>` (scalar uniform scale = `lod_assets.model_height * p.scale` to match the unit quad's vertical extent and the ortho-fit bake footprint) alongside the existing `lod3_instances`; after all LOD tiers are built, the new `impostor-bake` branch iterates `lod_assets.primitives`, calls `upload_impostor_pass_for_primitive(&primitive.full_mesh, &lod3_raw_instances, ...)` once per primitive (species label: `key` or `{key}_p{prim_idx}` for multi-primitive scatters), and accumulates returned keys in a per-refresh `impostor_live_keys: HashSet<String>`. After the groups loop, `self.retire_stale_impostor_passes(&impostor_live_keys)` drops passes whose meshes fell out of the active chunk set. The legacy `if lod3_count > 0 { <PBR-quad cascade> }` block now lives under `#[cfg(not(feature = "impostor-bake"))]` — stage 3d will delete it. Two new renderer methods: `current_view_proj() -> Mat4` (returns `cached_proj * cached_view` for external UBO fanout) and `update_all_impostor_cameras(view_proj, camera_pos)` (iterates `impostor_passes.values_mut()` and calls `pass.update_camera(&self.queue, vp, cam)` on each). Per-frame hook added to `EngineRenderAdapter::update_camera` so billboards re-aim their atlas-cell selection at the *current* camera direction every frame, not just at refresh time. Both default and `impostor-bake` configs compile clean; all 13 adapter impostor tests + 41 renderer impostor tests still green.
- **Tests landed**: 40 lib unit tests + 17 GPU integration tests = 57 new tests, all green. Default-features build unchanged.
- **Lessons captured**: WGSL `textureSample(texture, sampler, uv)` arg order (parser accepts wrong order but `naga::valid::Validator` rejects it with "ExpectedGlobalOrArgument" — always include the full validator in shader unit tests). Ortho half-width must be `sqrt(sx²+sz²)/2` to avoid clipping at non-axis angles. In-crate `[[bin]]` + `dep:` feature syntax cleanly breaks dependency cycles without a new crate. For monolithic renderers, shipping a standalone helper (`ImpostorPass`) before touching the core struct lets you land the draw path independently and de-risk the later integration. Monolithic renderers often have **two parallel render entry points** (here `render()` vs `draw_into()` — the editor only uses the latter) — always mirror the injection into both or half your callers silently get no-op behavior. `wgpu::Device` and `wgpu::Queue` are `Arc`-backed and cheaply `Clone`-able — use that to split disjoint `&mut self` borrows (registry + renderer) without `unsafe`. When installing GPU passes that persist across frames but own their own camera UBO, ship a fanout helper on the owner (`Renderer::update_all_impostor_cameras`) and call it in lockstep with the main camera update — otherwise stored UBOs silently drift from frame-to-frame camera state.
- **T7 stage 3d — legacy PBR-quad LOD3 fallback removed** (`tools/aw_editor/Cargo.toml` + `tools/aw_editor/src/viewport/engine_adapter.rs`): Flipped `impostor-bake` into `aw_editor`'s default feature set (`default = ["editor-core", "impostor-bake"]`) so `ImpostorPass` is now the canonical LOD3 scatter renderer. Deleted the `#[cfg(not(feature = "impostor-bake"))] if lod3_count > 0 { … }` block (~130 LOC: `first_lod3_source` texture-sharing waterfall, `add_model_sharing_texture` → `add_model_with_texture` → `add_model` cascade, per-bin bounds computation, instrumentation updates) and replaced it with a stub `_ = (lod3_count, &lod3_bins);` to keep the binning stats alive for instrumentation. Dropped the now-dead `primitive_lod2_sources: Vec<Option<String>>` vec and its write-site at the LOD2 tail (declaration + assignment). `primitive_card_sources` kept (still needed by LOD2's intra-tier texture sharing); `first_lod2_source` local kept (still needed for LOD2's `is_none()` cross-bin texture reuse). Graceful degradation: `--no-default-features` still compiles — LOD0/1/2 render normally, LOD3 silently renders nothing. Both default and `--no-default-features` builds: clean compile (only pre-existing `ChunkSplatMaps` warning). 13/13 adapter impostor tests still green. `astraweave-render` default features unchanged (other consumers not forced into `impostor-bake`).

### Blueprint Zone Editor & Blend Scene Integration ✅ (Feb 2026)
- **Scope**: Full polygon zone editor with .blend scene → terrain generation pipeline (9 phases)
- **Phase 1**: Core data model — `BlueprintZone`, `ZoneRegistry`, `BlendMask`, polygon math (24 tests)
- **Phase 2**: Decomposer extension — `heightmap_raster.rs` + `biome_pack.rs` rasterization (11 + 9 tests)
- **Phase 3**: Zone-scoped generation — `ZoneScatterGenerator` with Replica/Inspired modes (12 tests)
- **Phase 4**: Boundary blending — smoothstep auto-blend + `BrushMode::ZoneBlend` (16 tests)
- **Phase 5**: Asset browser — `BlendAssetScanner`, `AssetType::BlendScene`, editor preferences (8 tests)
- **Phase 6**: Blueprint panel — 2D canvas editor with polygon tools, undo/redo, zone inspector (17 tests)
- **Phase 7**: Viewport overlay — 3D wireframe zone visualization via debug lines (7 tests)
- **Phase 8**: System wiring — action processing, state sync, save/load, viewport overlay sync
- **Phase 9**: Integration tests + clippy (11 zone_scatter_e2e + 10 heightmap_raster_e2e tests)
- **Total new tests**: 125+
- **Crates**: `astraweave-terrain`, `astraweave-blend`, `aw_editor`

### Blend Import Pipeline ✅ (Feb 2026)
- **Scope**: Full `.blend` → BiomePack → Scatter pipeline (7 phases)
- **Phase 1**: Scene decomposition (`astraweave-blend` — decomposer, export script, options) — 63 tests
- **Phase 2**: BiomePack bridge format (`astraweave-terrain::biome_pack`) — 9 unit tests
- **Phase 3**: Texture processing pipeline (HDR→PNG, thumbnails, channel normalization) — 9 tests
- **Phase 4**: BiomeType extension + scatter integration (`BiomeConfig::from_biome_pack`, `ScatterConfig::from_biome_pack`)
- **Phase 5**: Editor UI — `BlendImportPanel` with full import workflow (13 tests, clippy clean)
- **Phase 6**: E2E integration tests — 12 tests covering manifest→BiomePack→BiomeConfig+ScatterConfig pipeline
- **Phase 7**: Architecture documentation updated
- **Reference**: `docs/current/ARCHITECTURE_REFERENCE.md` → Blend Import Pipeline section

### Miri Memory Safety Validation ✅ (Feb 3, 2026)
- **Scope**: All 4 crates with unsafe code validated
- **Results**: 977 tests, **ZERO undefined behavior** detected
- **Crates**: astraweave-ecs (386), astraweave-math (109), astraweave-core (465), astraweave-sdk (17)
- **Report**: `docs/current/MIRI_VALIDATION_REPORT.md`

### Fluids System ✅ (Jan 2026)
- 2,404 tests, SPH/pressure/viscosity/surface tension
- Grade: A+ (benchmark for all physics subsystems)

### Workspace Cleanup & WGPU 0.25 Migration ✅ (Nov 22, 2025)
- 377+ warnings fixed, zero-warning policy enforced
- `astraweave-render` fully migrated to wgpu 0.25

### Security Priority 1 ✅ (Nov 18, 2025)
- Network server vulnerabilities patched (C+ → A- grade)
- Editor 95% complete (Animation & Graph panels 100%)

### Phase 8.7: LLM Testing Sprint ✅ (Nov 17, 2025)
- 107 tests added, 100% pass rate
- Critical fix: `MockEmbeddingClient` determinism bug

### Phase 8.6: UI Testing Sprint ✅ (Nov 17, 2025)
- 51 tests added for core HUD logic, state management, edge cases

### Determinism Validation ✅ (Nov 1, 2025)
- Bit-identical replay, <0.0001 position tolerance
- 100-frame replay, 5-run consistency, 100 seeds tested

### Phase B Month 4: Integration Validation ✅ (Oct 31, 2025)
- 800+ integration tests across 106 test files
- 10 integration paths validated
- Performance SLA: 12,700+ agents @ 60 FPS proven

### Phase 7: LLM Validation ✅ (Jan 13, 2025)
- Hermes 2 Pro integration via Ollama
- 37-tool vocabulary, 4-tier fallback system, 5-stage JSON parser

### Phase 6: Real LLM Integration ✅ (Oct 14, 2025)
- 54 compilation errors resolved, all 6 AI modes functional
- Hermes 2 Pro connected, MockLLM eliminated

### Week 8 Performance Sprint ✅ (Oct 9-12, 2025)
- Frame time: 3.09ms → 2.70ms (-12.6%, 370 FPS)
- Tracy profiling integrated, spatial hash 99.96% fewer checks
- SIMD movement 2.08× speedup

### AI-Native Validation ✅ (Oct 13, 2025)
- 12,700+ agents @ 60 FPS, 6.48M validation checks/sec, 100% deterministic

### Astract Gizmo Sprint ✅ (Nov 2-3, 2025)
- React-style declarative UI framework, 7,921 LOC, 166/166 tests
- 5 tutorials, 4 API docs, performance benchmarks

---

## Performance Baselines

See `docs/current/MASTER_BENCHMARK_REPORT.md` for full data. Key numbers:

| Subsystem | Metric | Value |
|-----------|--------|-------|
| ECS | World creation | 25.8 ns |
| ECS | Entity spawn | 420 ns |
| ECS | Per-entity tick | <1 ns |
| AI Core Loop | Planning | 184 ns – 2.10 µs |
| GOAP | Cache hit | 1.01 µs |
| GOAP | Cache miss | 47.2 µs |
| Behavior Trees | Per-tick | 57–253 ns |
| Physics | Character move | 114 ns |
| Physics | Full tick | 6.52 µs |
| GPU Mesh | Vertex compression | 21 ns |
| SIMD Math | 10k entities | 9.879 µs (2.08× faster) |
| Frame (1k entities) | Total | 2.70 ms (370 FPS) |
| AI-Native | Agent capacity @ 60 FPS | 12,700+ |
| hello_companion | Classical mode | 0.20 ms |
| hello_companion | BehaviorTree mode | 0.17 ms |
| hello_companion | LLM mode | 3,462 ms |

---

## Validation Status

- `cargo check --workspace`: 130/130 members compile, 0 errors (verified 2026-06-10) — former known-build-issue crates (astraweave-author, rhai_authoring, ui_controls_demo, debug_overlay, astraweave-llm) all pass
- `hello_companion` demonstrates all 7 AI modes (feature-gated) (Phase 6+7) <!-- Source: CLAIMS_REGISTRY.md#ai-modes -->
- `cargo test -p astraweave-ecs` — comprehensive unit tests
- Net trio: 104/104 tests green (proto 59, server 41, client 4); camera: 25/25; cinematics: 341/341 (2026-06-10)
- CI validates SDK ABI, cinematics, and core crates
- **Miri**: 977 tests, 0 UB across 4 crates (ecs, math, core, sdk)
- **Determinism**: Bit-identical replay proven
- **Memory safety**: All unsafe code Miri-validated

---

**Version**: 0.10.1 | **Rust**: 1.89.0 | **License**: MIT
