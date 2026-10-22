# Security audit and hardening plan

> **2026-10-16 survey:** This survey is pinned to `f1109340`. #205, #214, #217, #244, #252, #275, #312, #316, #318, #325, #327, #329, and #331 are on main at `84d3888a`. Sentences below that say those pulls are not on main, or that `llm_dialogue.rs` is an undeclared orphan with `thread_rng`, are the survey, not the current tree.

> **Current as of 2026-10-21** (`main` @ `49d289eb` for the #388 and #397 sentences in this banner. The survey body was checked at `556a305c` unless a sentence names a later commit). #325 cleared the patched supply-chain advisories. #365 resolved RUSTSEC-2026-0370, 0371, and 0372 with no ignore. `eframe` is built without AccessKit. Editor file dialogs are `rfd` 0.17. The editor `kittest` feature is gone. #368 covers the six clicks with synthetic `RawInput`. #366 step 2, `egui_kittest`, waits on upstream AccessKit and zbus 6. The ignored leftovers are #305, #309, #310, and #321–#323. #308 no longer has an ignore: `wayland-scanner` 0.31.11 uses `quick-xml` 0.41.0. #306 no longer has an ignore: `mikktspace` uses its `glam` backend, so `nalgebra` 0.26 is gone. #320 no longer has an ignore: postcard is `default-features = false` plus `alloc`, so `atomic-polyfill` is gone. #320 is closed. The fix is #387. `rustls-webpki` is 0.103.16 and `webpki-roots` is 1.0.10. RUSTSEC-2026-0381 and RUSTSEC-2026-0382 are fixed with no ignore. #311 no longer has an ignore: `aw_trace_sync` parses trace front-matter with `yaml_serde`. #307 no longer has an ignore: `indicatif` 0.18 replaces `number_prefix`. #353 vendors `third_party/candle-core` (crates.io 0.8.4) pinned to `zip` 9.0.3. `zip` 1.1.4 is not in `Cargo.lock`. There is no audit ignore for RUSTSEC-2026-0367, 0368, or 0369. #324 is closed. The fix is #353. #217 closed #219 and #256: `openssl` is not in `Cargo.lock`, and `deny.toml` still bans it. #327 dropped `net` from the workspace `tokio` features. #326 is closed. `dig_sphere` applies a sphere `Subtract` brush (#329). The greybox golden is `b6a9d5a2c5da28a6` (#330). `gravity.rs` is deleted. #336 and #253 are closed. Greybox startup runs `run_aravath_slice` once (#275, #343). Each sword hit reuses that `sword_hp`. A checkpoint stores the demolition command log. #331 declares `llm_dialogue` and removes `thread_rng`. `recorded_line` is a pure helper with no non-test caller yet. `AIArbiter`, `GoapPlanner`, and `GravityManager` stay off the tick. #257 and #258 are closed. The fix is #382. #225 and #259 stay open. #253 is closed. The local field is required case `local_sim`. #340 closed #273 and #338. #333 closed #218. #254, #242, #314, #304, and #274 stay open. Profile `ci-pr` still excludes GPU render passes, lavapipe fluid smokes, and `test_qwen3_variants`. #388 (`43b1b48f`) closed #295. SEC-WEAVE-1 is on main: `queue_weave` returns `OutsideIsland` outside ±2048 cells and does not saturate. `meters_to_cell` returns `None` before the cast when the metre value is non-finite or `m * 1000` is below `i32::MIN as f32` or at or above `i32::MAX as f32`. `compile_legacy` drops those origins and a non-finite or non-positive-square wind delta. `push_ice` `debug_assert`s the island bound and still emits the sheet. In-range weaving replay bytes stay on `NullField`. #397 (`49d289eb`) closed #260. GitHub linked the issue to that pull. The mod host rejects an origin outside ±2048 before the cast is stored. A failed `run_one` drops that call's casts. An earlier script for the same agent that already returned `Ok` stays applied.
**Status:** Caps from this audit that already had a failing proof on `main` are landed. PR-side items stay on their branches.
**Date:** 2026-10-15
**Secret scanning:** GitHub secret scanning and push protection are enabled on this repository. No further settings change is required for SEC-SUPPLY-2.
**HEAD surveyed:** `f1109340` (`main`)
**Open PRs surveyed:** [#205](https://github.com/lazyxeon/AstraWeave/pull/205) `5048c630`, [#214](https://github.com/lazyxeon/AstraWeave/pull/214) `7a58612f`, [#252](https://github.com/lazyxeon/AstraWeave/pull/252) `5095f7eb`, [#213](https://github.com/lazyxeon/AstraWeave/pull/213) `61ebe2fa`
**Rule for the fixes:** hostile input is rejected. Bytes that already decode keep the same structures, so replay hashes of valid fixtures stay put.

This pass treats every external byte as hostile: save files, mod text, weave tables, model output, meshes, textures, cell RON, zip packs, and socket frames. "Wired" means a non-test caller on `main` reaches the function. A crate that only its own tests construct is compiled and dormant.

The 2025 plan at `docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md` and the seed in `docs/campaigns/security/S0_FINDINGS_SEED.md` are still the right map for three live holes (plaintext in-engine server, prompt logging, empty script allowlist). This document re-checks those and adds the parsers they did not cover. It does not replace `docs/architecture/security.md`.

---

## Landed on main

These are in the caps pull request. Valid fixtures still decode to the same bytes. Linux replay hashes match the pinned values (`combat_resolve` `975aae72e3c83a2335be05142eea2bf2ae4413d621fc5009ed6019f89e2a6e52`, plus `planner_pure`, `perception_world`, `gameplay_rng`, and `fixed64` from `cargo test -p astraweave-core --test replay_hash` and `cargo test -p astraweave-combat --test replay_hash`). The three-OS compare job diffs those same hashes; this change does not touch the hashed functions.

| ID | What landed |
|---|---|
| SEC-SAVE-1 | Declared `.awsv` length above 4 MiB is rejected before the payload buffer is allocated. LZ4 output above 16 MiB is rejected before decompress. |
| SEC-SAVE-2 | `decode_vitals` rejects more than 4,096 records. |
| SEC-NET-3 | `decode_msg` rejects an LZ4 declared output above 4 MiB. |
| SEC-ASSET-1 | glTF file URIs and cell asset paths go through `safe_under`. |
| SEC-MOD-4 | Script load and hot reload go through `safe_under`. |
| SEC-LLM-1 | Model clients keep a loopback URL only. `loopback_http_client` sets `no_proxy`, follows a redirect only when the next hop is loopback, and pins `localhost` to `127.0.0.1`. `allowing_remote` is the explicit opt-in: it keeps the caller URL and uses reqwest's default client, which honors `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` and follows redirects. `LOCAL_LLM_ALLOW_REMOTE=1` selects it in the integration example. |
| SEC-NET-1 | `ServerConfig::default` binds `127.0.0.1`. `spawn_server` refuses the published dev key outside debug and test builds. Release CLI requires `--shared-key-hex`. |
| SEC-NET-2 | The in-engine server ignores client actor ids and hello names. Outside debug/test, hello requires `AW_GAME_TOKEN` (not the literal `dev`). |

The compressed save cap is 4 MiB, not the 16 MiB sketch below, because the proof claims 8 MiB and must fail closed. The LZ4 message cap is 4 MiB for the same reason (the proof claims 16 MiB).

Nightly fuzzing is `.github/workflows/fuzz-nightly.yml`. It is not on pull requests. Each target runs for 120 seconds: `fuzz_awsv`, `fuzz_vitals`, `fuzz_gltf_json`, `fuzz_cell_ron`, `fuzz_decode_msg`.

## Findings, ranked

Severity is impact if the reachable caller is given a hostile file or socket, not whether a demo currently ships that caller.

| ID | Sev | Where | Wired on `main`? | One-line |
|---|---|---|---|---|
| SEC-NET-1 | Critical | `aw-net-server` default config | Yes, `spawn_server` | Listens on `0.0.0.0` with a signing key published in source |
| SEC-NET-2 | High | `astraweave-net` `GameServer` | Yes, examples | Client picks its actor; the `"dev"` token is optional |
| SEC-SAVE-1 | High | `aw-save` `read_any_version` | Yes, `aw_save_cli` | Header `data_len` is allocated before the file is checked |
| SEC-NET-3 | High | `aw-net-proto` `decode_msg` | Yes, matchmaking server | LZ4 output size is the attacker's `u32` |
| SEC-ASSET-1 | High | glTF URI + cell `load_asset` | Yes, editor and scene streaming | `../` is opened as a filesystem path |
| SEC-MOD-1 | High | `astraweave-scripting` `script_system` | Examples only | Script path is an unsandboxed file read; host uses raw pointers |
| SEC-LLM-1 | High | `LocalHttpClient` / Ollama constructors | Example + any `OLLAMA_URL` | A loopback URL is not enough: proxy env and redirects can still send the prompt. `loopback_http_client` closes that. `allowing_remote` is the explicit default-client opt-in |
| SEC-ASSET-2 | Medium | Pack Guard zip + `image` decode | `xtask` and glTF image path | Traversal is rejected; uncompressed size is not |
| SEC-SAVE-2 | Medium | `decode_vitals` | Public API, tests only | Record count is trusted up to the size of the string |
| SEC-LLM-2 | Medium | Quest + world-event JSON | Compiled, no production caller | Model JSON is applied to in-memory world flags with no size cap |
| SEC-LLM-3 | Medium | `sanitize_llm_prompt`, prompt logs | Dormant sanitizer; logs are live | S0-2 and S0-3 still match the code |
| SEC-SUPPLY-1 | Medium | `security-audit.yml` | CI | Advisory and license failures are warnings |
| SEC-SAVE-3 | Medium on merge | Scar blob on PR #205 | Not on `main` | Postcard cell list has no count cap |
| SEC-MOD-2 | Closed on main | `parse_template_toml` | Yes | Ids outside `u16` are rejected before `as u16` |
| SEC-WEAVE-1 | Closed (#388, #295) | `queue_weave` on main | Yes | Origins outside ±2048 cells return `OutsideIsland`. The conversion is not saturated |
| SEC-MOD-3 | Low on merge | PR #214 fuel vs RSS | Not on `main` | Operation budget is real; memory cap is arithmetic, not a process limit |

---

## 1. Save and persistence

### What exists

`aw-save` (`persistence/aw-save/src/lib.rs`) is the container. `#![forbid(unsafe_code)]`. On disk: magic `ASVS`, schema `u16`, codec byte, reserved byte, `data_len` `u32`, CRC32, then LZ4 of a postcard `SaveBundleV2`. Schema 2 is current. Schema 1 migrates in `read_awsv` (`persistence/aw-save/src/lib.rs:290-299`). Unknown versions return an error (`:109`, `:298`). Player ids are sanitized to ASCII alphanumerics, `-`, and `_` (`:342-351`), so a player id cannot walk out of the save root.

`astraweave-persistence-ecs` wraps that container. `#![forbid(unsafe_code)]`. `deserialize_ecs_world` (`astraweave-persistence-ecs/src/lib.rs:448-505`) postcard-decodes a `SerializedWorld` and spawns entities. Empty blobs return `Ok` (`:449-451`). `PersistencePlugin` registers `auto_save_system`, which is still a TODO (`:74-76`), and `replay_system`, which advances a tick counter and does not apply events (`:80-105`). The plugin is not registered by a game `App` on `main`. The production caller of the container is `tools/aw_save_cli`.

Combat vitals live in `astraweave-combat/src/save.rs`. `#![forbid(unsafe_code)]` on the crate. `combat_vitals` is a hex string: `u32` count, then 72-byte records (entity `u64` plus eight `i64`s). A bad hex digit, an odd length, or a body whose length is not `count * 72` returns `CombatError::BadSave` (`:83-95`, `:157-178`). Uppercase hex is rejected. There is no `unwrap` on this path. `read_combat_meta` also requires `combat_tables` to match the running table hash (`:125-132`). Callers on `main` are the crate's own tests. No game tick calls `decode_vitals` yet.

`astraweave-security::parse_{json,toml,ron}_limited` caps those three text formats at 10 MiB / 5 MiB / 5 MiB (`astraweave-security/src/deserialization.rs:10-12`). Zero production callers. The save path does not use it. Do not add a second limiter; either call this one for text sidecars or put the binary cap next to `read_any_version`.

### PR #205 greybox checkpoint

`aravath_runtime/src/save.rs` on `5048c630` writes a checkpoint through `SaveManager`. The edit log is capped before it is stored: 16_384 edits, coordinates inside ±2_048, 2 MiB of text, finite densities inside ±100, material id ≤ 16 (`aravath_runtime/src/save.rs:102-175`). The player record is capped at 256 bytes (`:76-78`). NaN and Inf are rejected, including hex `f32` bits. That layer does not panic on malformed text.

The scar blob beside it (`astraweave-veil/src/save_blob.rs:65-80` on the same commit) postcard-decodes a `Vec` of cells with a version check and a positive cell size, and no cell-count cap. `insert_into_meta` hex-encodes the blob into `SaveBundleV2.meta`. A hostile `.awsv` is expanded by `aw-save` before either of these parsers runs.

### Findings

**SEC-SAVE-1.** `read_any_version` (`persistence/aw-save/src/lib.rs:307-337`) does `let mut payload = vec![0u8; len];` with `len` taken from the header, then `read_exact`. A file whose header says 8 MiB and whose body is empty still enters that allocation. CRC is checked after the allocation. LZ4 then calls `lz4_flex::decompress_size_prepended`, which does `Vec::with_capacity(uncompressed_size)` from a `u32` prefix (`lz4_flex` 0.11.5 `block/decompress.rs:494-508`) before the stream is validated. Postcard after that will honor any `Vec` / `String` / `HashMap` length in the payload. `write_awsv` truncates `payload.len() as u32` (`:271`), so a legitimate save larger than 4 GiB would wrap; that is a separate integrity bug, not the hostile-input one.

PoC `poc_awsv_rejects_oversized_declared_payload` fails with `declared payload length was not rejected by a size cap: failed to fill whole buffer`.

**SEC-SAVE-2.** `decode_vitals` (`astraweave-combat/src/save.rs:83-113`) accepts any count whose body matches. 4_097 records decode. There is no panic. `Vec::with_capacity(len)` is reached only after the length check, so this is a size bomb, not a panic. `encode_vitals` casts `records.len() as u32` (`:68`), which truncates a huge in-memory slice.

PoC `poc_vitals_rejects_more_than_4096_records` fails with `decoded 4097 vitals records with no record cap`.

**SEC-SAVE-3 (PR #205 only).** Scar `from_postcard` has no cap on `wire.cells`. Fix it on that branch. Do not copy the parser onto `main`.

`file_name().unwrap()` in `write_or_update_index` and `scan_dir_for_meta` (`persistence/aw-save/src/lib.rs:217`, `:243`) runs on directory entries, not on the hostile payload. `format_description::parse(...).unwrap()` in `save` (`:57-59`) is a constant format string. Neither is the bomb.

### Unsafe on this path

None in `aw-save`, `astraweave-persistence-ecs`, or `astraweave-combat`. `lz4_flex` uses `unsafe { vec.set_len(decomp_len) }` inside the dependency after decompression. That is not an engine `unsafe` block. A wrong size from the attacker is still the engine's problem because the engine asks for that size.

---

## 2. Mod and scripting sandbox

### What exists on `main`

`astraweave-scripting` is Rhai. `ScriptEngineResource::default` sets real fuel and structural caps (`astraweave-scripting/src/lib.rs:74-78`):

- `max_operations` 50_000
- expression depth 64
- string, array, and map size 1_024

There is no wall-clock timeout on `script_system`. There is no process RSS cap. `ScriptLoader::load` (`astraweave-scripting/src/loader.rs:16-30`) reads any path it is given. `script_system` hot-reloads `CScript.script_path` with `std::fs::read_to_string` (`astraweave-scripting/src/lib.rs:195-219`) and compares `metadata.modified()` to a cache. That read is a filesystem escape, and the modified-time check is a wall clock on the tick. Production callers of `ScriptingPlugin` are `examples/scripting_advanced_demo` and `examples/scripting_playground`, not the Aravath demo.

The host API (`astraweave-scripting/src/api.rs:160-260`) is `log`, `spawn_prefab`, `despawn`, `set_position`, `apply_damage`, `play_sound`, `spawn_particle`, `raycast`, `find_path`. `play_sound` only prints the string (`astraweave-scripting/src/lib.rs:533-534`). It does not open the path. Prefab names outside `"enemy_grunt"` and `"crate"` still spawn an entity (`:121-132`). There is no template-id check on `main` because `queue_weave` is not on `main`.

`astraweave-security::execute_script_sandboxed` (`astraweave-security/src/lib.rs:372-405`) compiles and evals whatever source it is given. `allowed_functions` is never read. `max_memory_bytes` is stored and never passed to Rhai. The timeout is `tokio::time::timeout`, which is a wall clock and must not move onto a sim tick. `SecurityPlugin` is built only from tests. This is S0-3, still accurate. `astraweave-scripting` declares a dependency on `astraweave-security` and does not call it.

### PR #214

`astraweave-security/src/sandbox.rs` on `7a58612f` is a second, real sandbox, and it is the one the mod tick uses. Do not build a third.

What it already does:

- Fuel: `set_max_operations` default 50_000 (`sandbox.rs:50`, `:263`).
- Structural caps: string 4_096, array 1_024, map 256, variables 256, functions 64, expr depth 64, call levels 64 (`:51-57`).
- `set_max_modules(0)` and `disable_symbol` for `eval`, `import`, `export`, `print`, `debug` (`:271-276`).
- Manifest bytes, script bytes, script count, and mod count are capped (`:58-61`, `read_capped` at `:829`).
- Script and weave paths from a manifest are relative components only (`:752-816`).
- `timeout_ms` is 0 on the sim conversion (`:334-346`). The comment says the sim executor does not arm a wall clock. Keep that.
- Host commands are an allowlist: `apply_damage`, `configure`, `despawn`, `move_to`, `play_sound`, `queue_weave`, `set_position`, `sim_roll`, `spawn_particle`, `spawn_prefab` (`:35-46`).
- `queue_weave` (`astraweave-scripting/src/sim_mod.rs:254-279`) rejects a template id that does not fit in `u16`, a template the scratch does not know, and a full queue. Origins must fit in `i32`. The per-tick ceiling defaults to 64 (`sandbox.rs:65`).

What it does not do:

- `advertised_memory_bytes` (`sandbox.rs:162-167`) multiplies the structural caps. It is not a process limit and it is not enforced as one. That is SEC-MOD-3.
- The old `script_system` is still in `astraweave-scripting/src/lib.rs` on that branch, still taking raw `PhysicsWorld` and `NavMesh` pointers and still able to load a path if a caller puts one on `CScript`. The new tick comment says reloads come from `ScriptReloadQueue` and do not stat the filesystem. Both systems exist. The old one is the escape.

**SEC-MOD-2.** On #214 and #252, `parse_template_toml` cast the id with `as u16` and only then rejected engine ids, so a value such as `65544` became 8. On main, `mod_queue.rs:231-234` rejects `raw_id < 0` or `raw_id > u16::MAX` before the cast. Radius (`:276-279`) and node `a` (`:286-289`) use the same check-then-cast. `parse_rejects_engine_ids_and_accepts_modifier_data` locks `65536` and `-1`. `65535` stays id `65535`.

**SEC-MOD-4 (live).** `ScriptLoader::load("../secret.rhai")` returns `Failed to read script file: ../secret.rhai: No such file or directory`, not a traversal error. PoC `poc_script_loader_rejects_parent_segments` fails on that message.

### Unsafe on this path

`astraweave-security` is `#![forbid(unsafe_code)]`. The scripting crate is not.

| Site | What |
|---|---|
| `astraweave-scripting/src/api.rs:73-74` | `unsafe impl Send/Sync for PhysicsProxy` |
| `astraweave-scripting/src/api.rs:93` | `unsafe { &*self.ptr }` on the physics pointer |
| `astraweave-scripting/src/api.rs:143-144` | `unsafe impl Send/Sync for NavMeshProxy` |
| `astraweave-scripting/src/api.rs:154` | `unsafe { &*self.ptr }` on the nav pointer |
| `astraweave-scripting/src/lib.rs:466`, `:487`, `:514` | `unsafe { Entity::from_raw(...) }` when applying commands |
| `astraweave-scripting/src/events.rs:33` | `unsafe { Entity::from_raw(id) }` |

The pointer is taken while the resource is borrowed, the borrow is dropped, and the script runs (`astraweave-scripting/src/lib.rs:145-159`). Commands are queued and applied after the script, so a script cannot remove `PhysicsWorld` through the public command list during the raycast. The `unsafe` still aliases a resource the borrow checker cannot see. PR #214's sim host does not use these proxies for `queue_weave`. Leave the proxies unwired to mods.

Test-only `unsafe` in `astraweave-scripting/tests/mutation_resistant_comprehensive_tests.rs:29` is `Entity::from_raw` inside a test. It is not a runtime path.

---

## 3. Weave definitions

### What exists on `main`

`astraweave-weaving` is `#![forbid(unsafe_code)]`. On `main` at `43b1b48f`, `queue_weave` and `parse_template_toml` live in `mod_queue.rs`. `enqueue` returns `QueueError::OutsideIsland` when any origin axis is outside ±2048 (`mod_queue.rs:183-185`). `queue_weave` (`:215-225`) returns that result. The mod host rejects the same range before the cast is stored (`astraweave-scripting/src/sim_mod.rs:269-274`, #397). The 2026-10-16 survey sentence that this crate had no `queue_weave` describes `f1109340`, before #252. Abilities, anchors, combat-event queues, and the Aravath helper in `astraweave-weaving/src/aravath.rs` are still there. Particle counts stay capped (`MAX_PARTICLES_PER_ANCHOR` 500). Element schools used by combat go through `MAX_MODIFIERS` in `astraweave-weaving/src/damage.rs:32`.

`astraweave-destruction::enqueue_weave` is a destruction command queue, not the mod weave queue.

### PR #214 and PR #252

#252 (`5095f7eb`) carries `templates.rs`, `eval.rs`, `field.rs`, and `mod_queue.rs`. Built-in ids 1 through 7 are a closed match (`templates.rs:54-142`). Each template has at most 8 nodes. `WeaveScratch` stores 64 commands and 16 loaded templates in fixed arrays (`mod_queue.rs:9-12`). `weave_eval` sorts at most 64 commands and stops writes at `MAX_FIELD_WRITES` (64) (`eval.rs:132-165`, `field.rs:10`). Node `radius` is a `u8`. Duration is rejected outside `1..=10_000`. Node parameter `a` must fit in `i16`. Unknown `kind` strings error. This is bounded for a well-formed template.

**SEC-WEAVE-1.** On #252 (`5095f7eb`), `push_ice` computed `(origin[0] - radius) * CELL_MM` in `i32` and `queue_weave` accepted any `i32` origin. A large origin overflowed in debug and wrapped in release. #388 (`43b1b48f`) closed #295. GitHub linked the issue to that pull. On main, `enqueue` returns `QueueError::OutsideIsland` when any axis is outside ±2048 (`ISLAND_HALF_CELLS`, `mod_queue.rs:13` and `:183-185`). It does not saturate. `meters_to_cell` (`field.rs:40-50`) returns `None` when the metre value is non-finite, or when `m * 1000` is below `i32::MIN as f32` or at or above `i32::MAX as f32`, before the cast, then floors with `div_euclid`. `compile_legacy` returns `empty_compiled()` when `a` is outside the cube (`legacy.rs:55-57`) and when collapse endpoint `b` is outside (`:84-86`). `cell_origin` is `:168-178`. A non-finite wind delta, or one whose square is not strictly positive, fails `wind_delta_fits` (`:155-166`) and returns at `:97-99`. `1e-22` stays. `1e-23` underflows to 0. A dropped fortify, collapse, lower, or freeze returns at `legacy_geometry_kept` (`astraweave-gameplay/src/weaving.rs:57-58`) before the budget match at `:60-73`. A wind delta that `compile_legacy` drops returns at `:48-52` from `wind_direction`. The budget stays unchanged either way. `push_ice` `debug_assert`s the island bound (`eval.rs:233-238`) and stores the sheet at `:250-259`. The nav change is `:260-265`. The multiply is unchecked. The only caller already passed `enqueue`. The weaving replay case still hashes `NullField`.

**SEC-MOD-2.** The #252 snapshot carried the same truncating cast. On main the range check in `parse_template_toml` (`mod_queue.rs:231-234`) is already in place. Do not add a second parser.

On `43b1b48f`, `queue_weave` is on main. `mod_queue.rs` tests reject an origin outside the island with `OutsideIsland`. The #252 snapshot had not added that reject yet.

### Unsafe on this path

None in `astraweave-weaving`.

---

## 4. LLM output paths

Local models only. No cloud provider crate is a dependency of the engine. The clients that exist speak to a URL the caller supplies.

### What exists

| Client | Default URL | Restricts the host? |
|---|---|---|
| `Qwen3Ollama::localhost` / `::fast` | `http://localhost:11434` | Yes. `new` rewrites a non-loopback URL and stores `loopback_http_client` |
| `OllamaChatClient::new` | Caller-supplied | Yes. Same helper |
| `Phi3Ollama`, `Hermes2ProOllama` | `localhost` helpers | Yes. Same helper |
| `LocalHttpClient` | Caller-supplied | Yes. Posts to `{url}/v1/chat/completions` and can attach `Authorization: Bearer` only through that client |

`loopback_http_client` is the shared builder. It calls `no_proxy()`, so `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` are ignored. Its redirect policy follows a hop only when `is_loopback_http_url` accepts the next URL, and it stops a chain at 8 hops. The name `localhost` (optional trailing dot, any ASCII case) is a literal match. It is not resolved. The client pins that name to `127.0.0.1` and still uses the URL port. `::1` and `127.0.0.0/8` are accepted as addresses. Userinfo is rejected.

`allowing_remote` is the explicit opt-in. It keeps the caller URL. When that URL is not loopback, the stored client is reqwest's default client: the proxy variables apply and redirects are followed. When the URL is still loopback, `allowing_remote` uses `loopback_http_client` anyway. `LOCAL_LLM_ALLOW_REMOTE=1` selects the opt-in in `examples/llm_integration`. The tags probe in that example uses the same split.

`examples/llm_integration` builds `LocalHttpClient` from `OLLAMA_URL`, defaulting to `http://127.0.0.1:11434`, and will attach an API key when one is set. Nothing in the engine binds a listen socket for the model. Ollama is a separate process. The engine is the client.

`OLLAMA_MODEL` defaults differ by binary (`phi3:medium` in one path, `qwen3` names in others). That is configuration drift, not a network egress hole.

**SEC-LLM-1.** `poc_local_http_client_rejects_non_loopback_url` (feature `ollama`) stores `http://127.0.0.1:11434` for `https://api.openai.com`. `loopback_http_client_ignores_proxy_env_and_rejects_remote_redirects` covers the proxy variables and a redirect to a remote host.

### Can model text become a path, a shell command, a script, or a sim write?

Searched the LLM, quest, dialogue, memory, and coordination crates for `Command::new` and `std::process`. The only hits are `process::exit` in examples and disabled bins. No shell sink.

| Path | Compiled? | What model text becomes |
|---|---|---|
| `plan_parser::parse_llm_response` | Yes | A `PlanIntent` whose steps must match `ToolRegistry`. Unknown acts fail the parse (`astraweave-llm/src/plan_parser.rs:615` area, test rejects `FlyToMoon`). `ToolGuard` denies names `ExecuteCode`, `DeleteFile`, `ModifyWorld` (`astraweave-llm/src/tool_guard.rs:118-120`). `ToolGuard` has no production caller outside tests. The in-engine net server calls `validate_and_execute` on a client plan, not on raw model text. |
| `QuestLlmSystem` / `LlmQuestGenerator` | Yes (`astraweave-quests`) | `serde_json::from_str` into `LlmQuest` (`llm_quests.rs:429`). `validation_script: Option<String>` is stored (`:152`) and never evaluated. Callers are the crate's tests. |
| `WorldEventGenerator` | Yes (`astraweave-coordination`) | JSON becomes a `WorldEvent`. `apply_world_state_changes` (`world_events.rs:941-967`) replaces `faction_relations` and `resource_levels` and inserts other keys as bool flags. No production caller outside the crate's tests. The state object is the generator's own struct, not the ECS world. |
| `astraweave-dialogue/src/llm_dialogue.rs` | **No. Orphan file.** | Not declared from `astraweave-dialogue/src/lib.rs`. It imports `rand::thread_rng` (`llm_dialogue.rs:15`). Already tracked as deferred-failure [#225](https://github.com/lazyxeon/AstraWeave/issues/225). The compiled dialogue crate loads TOML through `toml_loader` and does not call a model. |
| `load_persona_zip` | Yes | Opens a caller-supplied path (`astraweave-persona/src/lib.rs:53-62`). Model text is not passed in. The zip member is selected by `by_name("persona_manifest.toml")`, so member names are not a traversal. `read_to_string` on that member has no byte cap. Callers: `examples/persona_loader` and tests. |
| PR #213 memory / RAG / persona | Not on `main` | `git grep` of `origin/pr213` under `astraweave-memory`, `astraweave-rag`, `astraweave-persona`, and `astraweave-npc` found no new `Command` or script eval. Persona still opens a path the caller passes. |

**SEC-LLM-2.** If a later PR wires `QuestLlmSystem` or `WorldEventGenerator` into the tick, model JSON becomes state with no byte cap and, for world events, no range check on the floats. `validation_script` must stay data. Do not pass it to Rhai. Quest ids are assigned with `Uuid::new_v4()` and timestamps with `Utc::now()` (`llm_quests.rs:433-434`). That is a wall clock. It is dormant. Do not move it onto the deterministic tick as part of the security fix.

**SEC-LLM-3.** `OllamaClient::complete` prints the full prompt and the URL (`astraweave-llm/src/lib.rs:175-184`). That is S0-2, still live. `sanitize_llm_prompt` prefixes `"SAFE: "` and returns the original text (`astraweave-security/src/lib.rs:358-364`). `LLMValidator.allowed_domains` includes `openai.com` and `anthropic.com` (`:163-166`). The validator has no production caller. Delete those domains when the validator is touched so a future caller cannot treat them as policy.

### Unsafe on this path

`astraweave-llm`, `astraweave-quests`, `astraweave-dialogue`, and `astraweave-coordination` lib crates are `#![forbid(unsafe_code)]`. `astraweave-coordination/benches/coordination_adversarial.rs` has an `unsafe` block in a benchmark, not on the event-generator path.

---

## 5. Asset loading

### What exists

`astraweave-asset` is `#![forbid(unsafe_code)]`. Default features include `gltf`.

`gltf_loader::load_uri_bytes` (`astraweave-asset/src/lib.rs:793-806`) accepts a `data:` base64 URI or passes the string to `std::fs::read`. `load_from_gltf_json` calls that for every buffer (`:738-743`) and `decode_image_from_gltf` calls it for image URIs (`:824-826`). No size cap, no jail, no extension allowlist.

`mesh_gltf::load_gltf` (`astraweave-render/src/mesh_gltf.rs:19-21`) calls `gltf::import(path)`. That loader resolves buffer and image URIs against the file's directory. The editor calls it from `tools/aw_editor/src/viewport/engine_adapter.rs:1299` (and `:1450`, `:2841`). A mesh the editor opens is a hostile file.

Cell streaming: `load_cell_from_ron` reads the whole file and `ron::from_str`s it (`astraweave-asset/src/cell_loader.rs:161-171`). `load_asset` joins `assets_root` with `asset_ref.path` and reads it (`:223-230`). `validate_mesh_format` checks a GLB magic or the substrings `"meshes"` and `"accessors"` (`:272-285`). It does not stop `../`. `astraweave-scene/src/streaming.rs:263` and `:271` call these two functions. The streamer is used by the world-partition examples and the slice runtime.

`decode_image_bytes` uses `image::load_from_memory` (`astraweave-asset/src/lib.rs:831-832`). `image` 0.25.8 `Limits::default` sets `max_alloc` to 512 MiB and leaves width and height unlimited (`image-0.25.8/src/io/limits.rs:49-55`). That stops a multi-gigabyte decode and still allows a 512 MiB frame stall.

Pack Guard is `tools/xtask/src/fetch_assets.rs`. `is_safe_relative` (`:1473-1486`) rejects empty paths, absolute paths, `..`, roots, and prefixes. `unpack_zip` (`:1624-1646`) bails on an unsafe member name and then `std::io::copy`s the member with no uncompressed budget and no entry-count cap. A zip-bomb member that stays inside the dest directory is extracted in full. This runs in the asset fetch tool, not in the frame loop. Tests in `tools/xtask/tests/fetch_assets.rs` already cover the traversal reject.

`safe_under` (`astraweave-security/src/path.rs`) is the existing jail. It is wired at three tool sites (editor scene serialization, texture gen, demo builder) and nowhere in the asset or cell loaders. Extend it. Do not write a third path checker. `is_safe_relative` stays the zip-member check; it does not look at the filesystem.

Terrain RON and JSON readers (`astraweave-terrain/src/zone_scatter.rs` around the `read_to_string` + `serde_json::from_str` helpers, and `regional_archetype_mask.rs:243-244`) also take a caller path with no size cap. They are the same class of hole as cell RON. PR #215 is the terrain streamer and should pick up the same jail rather than grow a new one.

### Findings

**SEC-ASSET-1.** Both PoCs fail:

- `poc_gltf_buffer_uri_rejects_parent_segments`: `Read uri ../secret.bin: No such file or directory (os error 2)`
- `poc_cell_asset_path_rejects_parent_segments`: `Failed to load mesh: /tmp/.../../outside.bin: No such file or directory (os error 2)`

**SEC-ASSET-2.** Pack unzip and persona-zip manifest reads have no uncompressed cap. Image decode relies on the crate default of 512 MiB and no dimension cap.

### Unsafe on this path

None in `astraweave-asset`. `meshopt` is C++ and is why the asset test binary links `libstdc++`. That is a build dependency, not an `unsafe` block on the loader path.

---

## 6. Networking and IPC

### Matchmaking trio (`net/aw-net-*`)

`aw-net-proto` is `#![forbid(unsafe_code)]`. Client input frames are HMAC-SHA256, verified before state changes, with `SignatureFailurePolicy::Kick` by default. That remediation (2026-06) still holds. Deliberate gaps that remain: no replay nonce, server-to-client messages unsigned.

**SEC-NET-1.** `ServerConfig::default` listens on `0.0.0.0:8788` and `0.0.0.0:8789` and uses `SigningKey::dev_default` (`net/aw-net-server/src/lib.rs:68-103`). The key is the ASCII constant `ASTRAWEAVE-DEV-KEY-NOT-FOR-PROD!` (`net/aw-net-proto/src/lib.rs:90-92`). The comment says it is not for production. The `Default` impl still installs it, and `spawn_server` is the real server entry. A peer who can reach the port can produce a valid tag. HTTP admin on `:8789` is still unauthenticated (the 2025 plan's P0 item 5).

**SEC-NET-3.** `decode_msg` for `Codec::PostcardLz4` (`net/aw-net-proto/src/lib.rs:257-266`) calls `lz4_flex::decompress_size_prepended` with no application cap, then postcard-decodes. The WebSocket layer's own cap is tungstenite 0.28's default `max_message_size` of 64 MiB (`tungstenite-0.28.0/src/protocol/mod.rs:101`). The uncompressed claim inside those 64 MiB can be a `u32`. The server does not set a tighter `WebSocketConfig`.

PoC `poc_postcard_lz4_rejects_oversized_declared_output` fails with `decode error: lz4: expected another byte, found none`. The library noticed the stream was short. It did not notice that the claimed output was 16 MiB.

`Codec::Bincode` (`:268-274`) uses `bincode` 2's standard config, which does not apply an application byte budget either.

### In-engine `astraweave-net`

`#![forbid(unsafe_code)]`. `GameServer::run_ws` binds whatever address the caller passes (`astraweave-net/src/lib.rs:533-535`). Examples use `127.0.0.1`. `handle_conn` accepts the tungstenite default and `serde_json::from_str`s the whole text (`:773-777`).

**SEC-NET-2 (S0-1, still true).** `ClientHello` checks `token != "dev"` only when a token is present, and only prints (`:785-788`). The name `"enemy"` maps the viewer onto the enemy entity (`:791-796`). `ClientInput` and `ClientProposePlan` take `actor_id` from the message and call `validate_and_execute` (`:823-860`). The connection is not bound to an actor. `validate_and_execute` is a gameplay check (bounds `(0,0)-(19,9)` in this server), not an authentication check. There is no per-connection byte counter below the 64 MiB websocket default.

### IPC

`astraweave-ipc` is `#![forbid(unsafe_code)]`. `handle_conn` (`astraweave-ipc/src/lib.rs:18-35`) JSON-decodes a `WorldSnapshot` and runs `RuleOrchestrator`. No size cap, no auth. The example binds `127.0.0.1:8088`. PR #252 edits this file; a size cap here should land with or before that PR so the rebase is small. The IPC crate does not touch the deterministic sim except by returning a `PlanIntent`.

### Unsafe on this path

None in `aw-net-proto`, `astraweave-net`, or `astraweave-ipc`.

---

## 7. Supply chain

### What exists

`.github/workflows/security-audit.yml` runs on pushes and pull requests to `main` and `develop`, and on a Sunday cron. `contents: read`. Two jobs:

- `cargo deny check bans` and `cargo deny check sources` fail the job.
- `cargo deny check advisories` turns timeout and exit code 1 into warnings (`security-audit.yml:63-76`).
- `cargo deny check licenses` turns failure into a warning (`:82-87`).
- `cargo audit` ignores `RUSTSEC-2020-0071` and turns exit code 1 into a warning that then `exit 0` (`:121-132`).

`deny.toml`:

- Licenses: an allow list that includes MIT, Apache-2.0, BSD, CC0-1.0, ISC, Zlib, MPL-2.0, Unicode, OFL-1.1, and several font/data licenses. Private workspace crates are ignored. Confidence 0.8.
- Bans: `openssl` and `openssl-sys` are denied. `rand_core < 0.6` is denied. `multiple-versions` is `warn`. Wildcards are `allow`.
- Sources: unknown registry and unknown git are `warn`, not `deny`. The only allowed registry is crates.io. `allow-git` is empty.
- Advisories: RustSec DB, 90-day staleness, `git-fetch-with-cli`. Ignored IDs are unmaintained-crate notices (`RUSTSEC-2024-0388`, `0384`, `0436`, `2025-0052`, `2023-0089`, `2025-0057`), not known exploited RCE. The file's first line still names the retired product title. Rename that comment when the file is next edited. Do not treat the ignore list as a vulnerability exception.

**SEC-SUPPLY-1.** A new advisory or a forbidden license does not fail this workflow. `unknown-git = "warn"` allows a git dependency to land with a warning.

**Closed by #217:** [#219](https://github.com/lazyxeon/AstraWeave/issues/219) and [#256](https://github.com/lazyxeon/AstraWeave/issues/256). `cargo deny check bans` was failing while `native-tls` pulled `openssl`. On `a66304dc`, reqwest, the websocket client, and `hf-hub` use rustls, `ort` does not enable `download-binaries`, and `Cargo.lock` has no `openssl` package. `deny.toml` still bans `openssl` and `openssl-sys`. This audit does not change those bans.

### Secrets hygiene

`.gitignore` ignores `.env`, `.env.local`, `.env.*.local`, `*.key`, `*.pem`, `*.p12`, `*.pfx`, `secrets/`, `private/`, and `auth/` (`.gitignore:60-70`). A search of the tree found no PEM private key and no `BEGIN PRIVATE` block. `net/certs/dev/` contains generation scripts and a README, not a committed key pair. TLS tests generate a certificate at runtime.

The signing constant in `SigningKey::dev_default` is committed on purpose and documented as such. It is not a leaked production secret. It is a default credential (SEC-NET-1).

GitHub secret scanning and push protection are enabled on this repository. No workflow in `.github/workflows` runs a second scanner. That is enough for SEC-SUPPLY-2.

### Fuzz targets that already exist

`cargo-fuzz` targets already live under `astraweave-asset/fuzz`, `astraweave-llm/fuzz`, `astraweave-net/fuzz`, `astraweave-ecs/fuzz`, and `crates/astraweave-blend/fuzz`. No workflow invokes them. They do not cover `aw-save`, combat vitals, mod manifests, or weave TOML.

---

## Proposed fixes

Each fix rejects input. None of them reorder fields, bump `SAVE_SCHEMA_VERSION`, or add a clock.

### Save

1. In `read_any_version`, reject `data_len` above a constant (proposal: 16 MiB compressed) before the `vec![0u8; len]`. Reject the LZ4 uncompressed prefix above the same order of magnitude (proposal: 64 MiB) by reading the `u32` and returning `Err` before `decompress_size_prepended`. Keep CRC and the schema match as they are.
2. In `decode_vitals`, reject `len > 4_096` after the stride check and before `Vec::with_capacity`. 4_096 is above every vitals fixture in `astraweave-combat` (the tests use one record). If a later demo save is larger, raise the constant above that save. Do not lower it onto a fixture.
3. On PR #205, cap `wire.cells` in `from_postcard` the same way. Leave the edit-log caps already in `aravath_runtime/src/save.rs` alone.
4. Point text sidecars (`index.json`, dialogue TOML, cell RON) at `parse_*_limited` or the same numbers. Do not write a new JSON limiter.

### Mods and weaves

1. Do the sandbox work on PR #214. Do not add a third Rhai engine on `main`.
2. Change `parse_template_toml` so the id is range-checked as `i64` before the `u16` cast: `1..=7` stays reserved, `8..=u16::MAX` is a mod id, anything else errors. In-range ids keep today's bytes.
3. Reject `queue_weave` origins outside the island (the greybox checkpoint uses ±2_048; use that or the terrain streamer's chunk bounds, one constant). `push_ice` then only multiplies values that cannot overflow `i32`. Do not saturate.
4. Keep `timeout_ms = 0` and the operation budget. Document `advertised_memory_bytes` as a structural ceiling, not an RSS cap. A process RSS cap is a tool-side `setrlimit` around the editor, not a sim tick.
5. Stop `script_system` on `main` from reading `script_path` off disk. Reloads go through the queued, hashed swap #214 already describes. Until #214 merges, `ScriptLoader::load` should refuse `..`, absolute paths, and prefixes using `safe_under` or `is_safe_relative`.

### LLM

1. Add one helper, `require_loopback_http_url`, and call it from `Qwen3Ollama::new`, `OllamaChatClient::new`, `Phi3Ollama::new`, `Hermes2ProOllama::new`, and `LocalHttpClient::new`. Allow `127.0.0.1`, `localhost`, and `[::1]` only. Reject `https://api.openai.com` and any other host. Apply the same check to `OLLAMA_URL` at the example boundary.
2. Remove the bearer-key constructor from the path that the example uses, or make `with_api_key` return an error. No cloud provider.
3. Leave `validation_script` unread by Rhai. Bound the JSON `complete()` return (proposal: 64 KiB) before `serde_json::from_str` in the quest and world-event parsers when those parsers are wired. They are not wired today; the cap can land with the wiring PR, not before.
4. Stop logging full prompts (S0-2) in the same change as the URL check. That does not affect replay hashes. It does affect stderr.
5. #331 registers `llm_dialogue` and removes `thread_rng`. `recorded_line` is a pure helper intended as the sim boundary. It does not draw, and it is not wired in yet. `generate_response` stays off the tick. #225 stays open as that record.

### Assets

1. Resolve glTF buffer and image URIs with `safe_under` against the glTF file's directory. `data:` URIs stay in memory and get a byte cap (proposal: 16 MiB decoded).
2. `load_asset` refuses a path that is not `safe_under(assets_root, relative)`.
3. `unpack_zip` and `load_persona_zip` stop after a total uncompressed budget (proposal: 512 MiB) and an entry cap (proposal: 10_000). Traversal behavior stays `is_safe_relative`.
4. Set `image::Limits` explicitly: `max_alloc` 64 MiB, and a width and height cap (proposal: 8192) on the glTF image decode. Editor loads of the current CC0 packs must be measured first; if a pack image is larger, raise the constant above that image rather than decoding it unbounded.

### Networking

1. `ServerConfig::default` binds `127.0.0.1`. Starting on `0.0.0.0` requires an explicit flag. `SigningKey::dev_default` refuses to boot unless a `--dev-key` flag is set. Production loads the key from the environment or the secrets crate. This changes who can connect. It does not change the bytes of a signed frame produced with a real key.
2. `decode_msg` uses the same uncompressed cap as `aw-save`.
3. `GameServer::handle_conn` sets `WebSocketConfig.max_message_size` to 1 MiB, requires a real authenticator before any `ClientInput`, and ignores `actor_id` that is not the id bound at hello. The `"dev"` string check goes away.
4. `astraweave-ipc::handle_conn` uses the same 1 MiB cap. It stays on loopback in the example.

### Supply chain

1. #217 and #219 are on `main` and closed. `security-audit.yml` runs bans, sources, advisories, and licenses with `set -e`, so a deny failure fails the job. On `a66304dc`, `unknown-git` is `deny` and `unknown-registry` is `warn`. Leave the unmaintained-crate ignore list.
2. Secret scanning and push protection are already enabled. Leave them on.

---

## Fuzzing plan

PR CI does not run fuzzing today, and it should stay that way. The existing `fuzz/` crates are not referenced by any workflow. Adding them to `pull_request` would add a non-deterministic, open-ended job to every review.

Add `.github/workflows/fuzz-nightly.yml` only:

- Triggers: `schedule` (nightly) and `workflow_dispatch`. Not `pull_request`, not `push`.
- One job, `ubuntu-latest`, `timeout-minutes: 30`.
- Install `cargo-fuzz` once, cache it.
- For each target, `cargo fuzz run <name> -- -max_total_time=120`. A crash fails the nightly job and should become an issue. It does not fail a feature PR.
- Commit a tiny seed corpus (valid minimal files plus the PoC inputs from this audit). Do not commit a multi-megabyte corpus.

Targets, in this order, each a thin wrapper that returns on `Err` and panics only on a caught panic or a timeout inside the parser:

| Target | Crate | Function | When it can land |
|---|---|---|---|
| `fuzz_awsv` | `aw-save` | `read_awsv` on a temp file, or a `pub(crate)` test-visible entry if we refuse to fuzz through the filesystem | With the save cap, on `main` |
| `fuzz_vitals` | `astraweave-combat` | `decode_vitals` | With the vitals cap, on `main` |
| `fuzz_gltf_json` | `astraweave-asset` | `load_first_mesh_and_material` | With the URI jail, on `main` |
| `fuzz_cell_ron` | `astraweave-asset` | `load_cell_from_ron_sync` on bytes written to a temp file | With the cell cap, on `main` |
| `fuzz_mod_manifest` | `astraweave-security` | `parse_manifest_str` | Only after #214, on that branch's sandbox |
| `fuzz_weave_toml` | `astraweave-weaving` | `parse_template_toml` | Only after #214/#252, one target, one parser |

`fuzz_awsv` and `fuzz_vitals` are the first two because the PoCs already show the missing caps. Do not fuzz `script_system`: it needs a `World` and a filesystem. Fuzz the manifest parser, which is pure.

Nightly fuzzing does not change replay hashes. It does not run in the determinism workflow.

---

## Replay hashes

The fixes above are reject-only on inputs that are not in the replay fixtures.

- Save schema stays `SAVE_SCHEMA_VERSION = 2`. Field order of `SaveBundleV2`, `WorldState`, and `VitalsRecord` stays put. A file that decodes today and is under the new caps decodes to the same postcard value.
- Combat fixtures use a single vitals record. The 4_096 cap is above that. `encode_vitals` sort-by-entity stays.
- Greybox edit encoding on #205 (hex `f32` bits, the 16_384 cap) is not part of this change.
- Weave replay bytes (`AWWEAV2`, origins near the origin cell, templates 1 through 4) stay. Out-of-range origins error instead of wrapping, so an in-range cast is bit-identical. Do not saturate.
- Template ids that already fit in `u16` and are greater than 7 parse to the same id. The truncation bug is a reject of ids that do not fit, which never had a stable hash.
- Operation fuel is a counter. It is deterministic. Do not add `tokio::time::timeout` or `Instant` to the mod tick. #214 already sets `timeout_ms` to 0 for this reason. Keep it.
- The LLM URL check and the prompt-log removal sit outside the tick. They do not hash.
- `Uuid::new_v4()` and `OffsetDateTime::now_utc()` inside `SaveManager::save` already make two saves of the same world differ on disk. That is pre-existing and outside the sim hash. Do not "fix" it in the security PR by introducing a new clock. A deterministic save name is a separate change and needs its own replay note.
- After the fix PR, re-run the existing determinism replay workflow on the fixtures it already hashes. This audit does not change those fixtures, so this PR itself has nothing to rehash.

---

## Merge order

This pull request is the review draft. It does not merge. The failing PoCs are the evidence. Do not delete them to make CI green. Land the caps, and the same tests go green.

Open feature PRs that touch the same files, and the order to use:

1. **Review this draft.** Open the issues in the table below. The user turns on secret scanning and push protection.
2. **PR #217 / issue #219** merged as `a66304dc`. The openssl ban stayed. The lockfile no longer contains the package. A later security change can edit `deny.toml` without fighting that branch.
3. **Small `main` PR: parsers that exist today.** `aw-save` length cap, `decode_vitals` count cap, `decode_msg` LZ4 cap, `safe_under` on glTF URIs and `load_asset`, `ScriptLoader` path reject, loopback check on the LLM clients, `GameServer` message cap. No sandbox rewrite. Re-run replay fixtures.
4. **PR #214 before any new sandbox work.** Fix SEC-MOD-2 (id cast) and keep the old `script_system` from reading the disk, on that branch. It already has the fuel limit, the allowlist, and `timeout_ms = 0`.
5. **PR #252 rebases onto #214** if both are still open, because both carry `astraweave-weaving/src/mod_queue.rs`. Fix SEC-WEAVE-1 (origin range) once, on the branch that owns `push_ice` after the rebase. Do not fork `parse_template_toml`.
6. **PR #205 rebases onto step 3** so the greybox checkpoint inherits the container cap. Its edit-log parser is already bounded. Add the scar-cell cap (SEC-SAVE-3) on #205, not on `main`. Child branches of #205 (#275, #264, #263, #244) rebase after #205 does.
7. **PR #213 rebases onto step 3** so the memory work inherits the loopback check. Do not register the orphan dialogue file. Do not let RAG text become a path.
8. **PR #215 (terrain streamer)** uses the cell-path jail from step 3 instead of a new one.
9. **PR #252's IPC edit** picks up the 1 MiB cap from step 3.
10. Gameplay and rendering drafts (#243, #216, #253, #275, #263, #264, #244) do not need to wait, except where they sit on #205 or edit `aw-save` / `mod_queue.rs` / `astraweave-llm/src/lib.rs`. Those rebase. Dependabot bumps (#199–#204) stay separate from the security PR.

---

## Deferred-failure issues to open

Not opened by this change. Label `deferred-failure`. Owner is `@lazyxeon` (repository owner). Reassign if someone else takes the fix. "Plan" is the patch, not a calendar.

| ID | Title to open | Owner | Plan | Already tracked? |
|---|---|---|---|---|
| SEC-SAVE-1 | Cap `.awsv` declared length and LZ4 output | `@lazyxeon` | Reject `data_len` and the LZ4 `u32` before allocation. Schema stays 2. Turn `poc_awsv_rejects_oversized_declared_payload` green. | No |
| SEC-SAVE-2 | Cap `combat_vitals` record count at 4_096 | `@lazyxeon` | Reject in `decode_vitals` after the stride check. Raise the constant only if a real fixture is larger. Turn `poc_vitals_rejects_more_than_4096_records` green. | No |
| SEC-SAVE-3 | Cap scar-field cells on PR #205 | `@lazyxeon` | Cap `wire.cells` in `from_postcard` on the greybox branch. Do not copy the parser to `main`. | No |
| SEC-MOD-1 | Script path read and raw host pointers | `@lazyxeon` | On #214, keep fuel and `timeout_ms = 0`. Stop the old `script_system` from reading `script_path`. Leave `PhysicsProxy` off the mod tick. | S0-3 is the dormant allowlist; this row is the live scripting crate |
| SEC-MOD-2 | `parse_template_toml` truncates ids with `as u16` | `@lazyxeon` | Range-check the `i64` before the cast, on the single branch that owns the function after #214/#252 rebase. | Yes. On main. `mod_queue.rs:231-234` rejects an id outside `u16` before the cast. |
| SEC-MOD-4 | `ScriptLoader` accepts `..` | `@lazyxeon` | Reject via `safe_under` or `is_safe_relative`. Turn `poc_script_loader_rejects_parent_segments` green. | No |
| SEC-WEAVE-1 | Weave origin can overflow the milli conversion | `@lazyxeon` | Reject origins outside the island in `queue_weave`. Do not saturate. In-range replay bytes stay. | Yes. #295 is closed. GitHub linked it to #388 |
| SEC-LLM-1 | Model client accepts a non-loopback URL | `@lazyxeon` | `is_loopback_http_url` plus `loopback_http_client` (`no_proxy`, loopback-only redirects, `localhost` pinned to `127.0.0.1`). `allowing_remote` keeps a remote URL on reqwest's default client. | No |
| SEC-LLM-2 | Quest and world-event JSON is an unbounded in-memory write | `@lazyxeon` | Keep both off the tick. When wiring, cap the model string at 64 KiB and do not eval `validation_script`. | No |
| SEC-LLM-3 | Full prompts are logged; sanitizer prefixes `SAFE:` | `@lazyxeon` | Delete the prompt dump. Leave the sanitizer dormant or make it actually reject. Drop `openai.com` and `anthropic.com` from the default domain list. | S0-2 |
| SEC-ASSET-1 | glTF and cell paths accept `..` | `@lazyxeon` | `safe_under` for URIs and `load_asset`. Turn both asset PoCs green. | No |
| SEC-ASSET-2 | Zip and image bombs have no tight budget | `@lazyxeon` | Uncompressed and entry caps on `unpack_zip` and persona zips. Explicit `image::Limits` after measuring CC0 pack sizes. | No |
| SEC-NET-1 | Server default is `0.0.0.0` plus the published dev key | `@lazyxeon` | Loopback unless flagged. Refuse `dev_default` unless `--dev-key`. | Extends the 2025 plan's admin-port note |
| SEC-NET-2 | In-engine server trusts the client | `@lazyxeon` | Authenticate, bind `actor_id`, 1 MiB message cap. Remove the `"dev"` print. | S0-1 |
| SEC-NET-3 | `decode_msg` honors the attacker's LZ4 size | `@lazyxeon` | Same uncompressed cap as `aw-save`. Turn `poc_postcard_lz4_rejects_oversized_declared_output` green. | No |
| SEC-SUPPLY-1 | Advisory and license `cargo deny` failures were warnings | `@lazyxeon` | `security-audit.yml` fails the job on bans, sources, advisories, and licenses. `unknown-git` is `deny`. `unknown-registry` is `warn`. #219 is closed. | #219 covered the openssl ban failure |
| SEC-SUPPLY-2 | Secret scanning and push protection | `@lazyxeon` | Enabled on the repository. No code change. Do not refile. | Done |
| — | `llm_dialogue.rs` was an orphan and named `thread_rng` | #331 on main | The module compiles. `recorded_line` is an unwired pure helper and does not draw. `generate_response` stays off the tick. #225 stays open. | [#225](https://github.com/lazyxeon/AstraWeave/issues/225) |
| — | `cargo deny` failed on `native-tls` / `openssl` | closed | #217 switched TLS to rustls and dropped the `ort` download feature. The ban stayed. | [#219](https://github.com/lazyxeon/AstraWeave/issues/219) |

The six PoCs in this pull request fail on purpose. They are the regression net for SEC-SAVE-1, SEC-SAVE-2, SEC-NET-3, SEC-ASSET-1, SEC-MOD-4, and SEC-LLM-1. Deleting them is not the fix.

---

## Proof-of-concept tests

Run from the repo root. Each one is expected to fail until the matching cap exists.

```text
cargo test -p aw-save --lib poc_awsv_rejects_oversized_declared_payload -- --nocapture
cargo test -p astraweave-combat --test security_poc -- --nocapture
cargo test -p aw-net-proto --test security_poc -- --nocapture
cargo test -p astraweave-asset --test security_poc -- --nocapture
cargo test -p astraweave-scripting --test security_poc -- --nocapture
cargo test -p astraweave-llm --features ollama --lib poc_local_http_client_rejects_non_loopback_url -- --nocapture
```

Observed on 2026-10-15:

| Test | Failure |
|---|---|
| `poc_awsv_rejects_oversized_declared_payload` | `declared payload length was not rejected by a size cap: failed to fill whole buffer` |
| `poc_vitals_rejects_more_than_4096_records` | `decoded 4097 vitals records with no record cap` |
| `poc_postcard_lz4_rejects_oversized_declared_output` | `decode error: lz4: expected another byte, found none` |
| `poc_gltf_buffer_uri_rejects_parent_segments` | `Read uri ../secret.bin: No such file or directory (os error 2)` |
| `poc_cell_asset_path_rejects_parent_segments` | `Failed to load mesh: .../../outside.bin: No such file or directory (os error 2)` |
| `poc_script_loader_rejects_parent_segments` | `Failed to read script file: ../secret.rhai: No such file or directory (os error 2)` |
| `poc_local_http_client_rejects_non_loopback_url` | `model client stored a non-loopback endpoint: https://api.openai.com` |

No other test in those crates was run as a suite. Pre-existing red CI already tracked elsewhere (#218 audio, #220 clippy, #242 terrain goldens, #254 nextest, #219 cargo-deny) was not re-run and is not claimed fixed.
