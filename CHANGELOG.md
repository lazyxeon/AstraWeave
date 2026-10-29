# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html) for the workspace package in the root `Cargo.toml`. Git tags `v0.4.0`, `v0.3.0`, `v0.2.0`, and `v0.1.0` were never published; the pre-2026 sections below are archival notes only.

## [Unreleased]

The workspace package is still version 0.4.0. [SECURITY.md](SECURITY.md) calls that the supported line. There is no `v0.4.0` git tag. The notes in this section are changes on `main` after 2025-11-18, grouped by the UTC month of the merge or commit. The dated `[0.4.0]` heading further down is the archival note for that day, not a release tag.


### 2026-10

#### Added
- feat(combat): deterministic Q32.32 damage resolver ([#212](https://github.com/lazyxeon/AstraWeave/pull/212)).
- feat(terrain): integer ground, command-log deformation, one streamer ([#215](https://github.com/lazyxeon/AstraWeave/pull/215)).
- feat(destruction): replay, prefracture, and collapse bounds ([#243](https://github.com/lazyxeon/AstraWeave/pull/243)).
- feat(math): canonical Q32.32 Fixed64 and cross-OS replay case ([#245](https://github.com/lazyxeon/AstraWeave/pull/245)).
- feat: data-driven weaving framework (draft, do not merge) ([#252](https://github.com/lazyxeon/AstraWeave/pull/252)).
- feat(aravath): bridge-collapse reroute on the orchestration head ([#263](https://github.com/lazyxeon/AstraWeave/pull/263)).
- feat(aravath): wire sword hits through the combat slice ([#275](https://github.com/lazyxeon/AstraWeave/pull/275)).
- feat(qa): replay the Aravath slice across three OS ([#280](https://github.com/lazyxeon/AstraWeave/pull/280)).
- feat(causality): deterministic event ring and causal_bridge replay ([#286](https://github.com/lazyxeon/AstraWeave/pull/286)).
- feat(ai): zero-cost decision ring and headless inspector ([#287](https://github.com/lazyxeon/AstraWeave/pull/287)).
- feat: local field on main, greybox host, weaving stamps ([#330](https://github.com/lazyxeon/AstraWeave/pull/330)).
- feat(render): enable Bruneton on the fog-and-shafts path after 4.474 ms ([#412](https://github.com/lazyxeon/AstraWeave/pull/412)).
- feat(render): draw scene normals and motion when enabled ([#416](https://github.com/lazyxeon/AstraWeave/pull/416)).

#### Changed
- build(deps): bump benchmark-action/github-action-benchmark from 1.20.7 to 1.22.2 ([#204](https://github.com/lazyxeon/AstraWeave/pull/204)).
- Aravath demo: CC0 characters, streamed world, and sword combat ([#205](https://github.com/lazyxeon/AstraWeave/pull/205)).
- ci(aravath): Windows release build of aravath_greybox ([#206](https://github.com/lazyxeon/AstraWeave/pull/206)). Merged into `cursor/aravath-greybox-b3de`, not `main`.
- Harden the Aravath Windows download ([#207](https://github.com/lazyxeon/AstraWeave/pull/207)). Merged into `cursor/aravath-greybox-b3de`, not `main`.
- Add the engine production roadmap and determinism gate ([#208](https://github.com/lazyxeon/AstraWeave/pull/208)).
- Add deterministic fate-scars to volumetric fog and enemy perception ([#209](https://github.com/lazyxeon/AstraWeave/pull/209)). Merged into `cursor/aravath-greybox-b3de`, not `main`.
- Determinism gate: ordered perception and a planner for every companion ([#210](https://github.com/lazyxeon/AstraWeave/pull/210)).
- WS-D2 and WS-STAB: seeded sim RNG and green debug CI ([#211](https://github.com/lazyxeon/AstraWeave/pull/211)).
- WS-MOD: sandboxed Rhai mods and tick-boundary hot reload ([#214](https://github.com/lazyxeon/AstraWeave/pull/214)).
- WS-RENDER-ENV-A: showcase IBL, PBR maps, deterministic time of day ([#216](https://github.com/lazyxeon/AstraWeave/pull/216)).
- ci: burn down pre-existing failures without widening tolerances ([#217](https://github.com/lazyxeon/AstraWeave/pull/217)).
- ci: tier pull-request checks and keep cross-OS replay hashes ([#223](https://github.com/lazyxeon/AstraWeave/pull/223)).
- Traversal: tick sim, hybrid climb, and the Aravath course ([#244](https://github.com/lazyxeon/AstraWeave/pull/244)).
- docs: open-world AI orchestration archaeology (Phase 1, review before code) ([#251](https://github.com/lazyxeon/AstraWeave/pull/251)).
- docs: causal world-state design for review ([#276](https://github.com/lazyxeon/AstraWeave/pull/276)).
- docs: plan autonomous gameplay QA for the Aravath slice ([#277](https://github.com/lazyxeon/AstraWeave/pull/277)).
- docs: semantic AI decision debugger design (review before code) ([#278](https://github.com/lazyxeon/AstraWeave/pull/278)).
- Keep physics and nav off the gameplay script tick ([#318](https://github.com/lazyxeon/AstraWeave/pull/318)).
- docs: correct engine status and guard it with a front-door lint ([#319](https://github.com/lazyxeon/AstraWeave/pull/319)).
- Clear patched supply-chain advisories without moving replay hashes ([#325](https://github.com/lazyxeon/AstraWeave/pull/325)).
- Keep tokio net out of the Aravath greybox graph ([#327](https://github.com/lazyxeon/AstraWeave/pull/327)).
- docs: greybox tokio net is off and the sword hit calls the slice ([#328](https://github.com/lazyxeon/AstraWeave/pull/328)).
- Aravath showcase: bridge reroute, brush digs, integer columns ([#329](https://github.com/lazyxeon/AstraWeave/pull/329)).
- docs: models, GOAP, and gravity stay off the sim tick ([#337](https://github.com/lazyxeon/AstraWeave/pull/337)).
- docs: greybox steps the bridge agent and digs with a brush ([#339](https://github.com/lazyxeon/AstraWeave/pull/339)).
- docs: fix recorded-line, weaving users, in-progress lint, roadmap history ([#341](https://github.com/lazyxeon/AstraWeave/pull/341)).
- ci: review pull requests with CodeRabbit, including drafts ([#344](https://github.com/lazyxeon/AstraWeave/pull/344)).
- docs: local_sim is the sixteenth replay case ([#345](https://github.com/lazyxeon/AstraWeave/pull/345)).
- docs: greybox sword cache and checkpoint demolition log (#343) ([#346](https://github.com/lazyxeon/AstraWeave/pull/346)).
- docs: parity fix and workspace-test skip cuts (#340, #333) ([#347](https://github.com/lazyxeon/AstraWeave/pull/347)).
- Check out Aravath LFS blobs in the toolchain job ([#348](https://github.com/lazyxeon/AstraWeave/pull/348)).
- docs: nineteen required replay cases (#332) ([#349](https://github.com/lazyxeon/AstraWeave/pull/349)).
- Stop per-step physics allocations in the modified-collider buffer ([#350](https://github.com/lazyxeon/AstraWeave/pull/350)).
- Measure astraweave-localsim on benchmark shard 0 ([#351](https://github.com/lazyxeon/AstraWeave/pull/351)).
- Point candle-core at patched zip 9.0.3 ([#353](https://github.com/lazyxeon/AstraWeave/pull/353)).
- docs: toolchain LFS checkout and localsim benchmark shard ([#354](https://github.com/lazyxeon/AstraWeave/pull/354)).
- Measure renderer lifecycle RSS after a warm cycle ([#355](https://github.com/lazyxeon/AstraWeave/pull/355)).
- Split bench targets that exceed the 600s cap ([#356](https://github.com/lazyxeon/AstraWeave/pull/356)).
- Update the query pipeline before the 8-ray bench ([#357](https://github.com/lazyxeon/AstraWeave/pull/357)).
- Register schedule stages and clear clippy -D warnings ([#360](https://github.com/lazyxeon/AstraWeave/pull/360)).
- Split benchmark shards 2 and 4 under the 600s cap ([#361](https://github.com/lazyxeon/AstraWeave/pull/361)).
- docs: status after vendored Rapier, shard splits, and editor access ([#362](https://github.com/lazyxeon/AstraWeave/pull/362)).
- Document the vendored rapier3d buffer-reuse patch ([#363](https://github.com/lazyxeon/AstraWeave/pull/363)).
- test: click editor toolbar and menus without egui_kittest (#366 step 1) ([#368](https://github.com/lazyxeon/AstraWeave/pull/368)).
- docs: #360 closed the schedule expect ([#373](https://github.com/lazyxeon/AstraWeave/pull/373)).
- bench(fluids): flat-sample fluid_baseline_50k under the 600s cap ([#374](https://github.com/lazyxeon/AstraWeave/pull/374)).
- test(terrain): drop the forest LOD host-clock assert ([#375](https://github.com/lazyxeon/AstraWeave/pull/375)).
- docs: #367 raycast bench is on main ([#376](https://github.com/lazyxeon/AstraWeave/pull/376)).
- bench(render): fit shard 13 under the 600s cap ([#377](https://github.com/lazyxeon/AstraWeave/pull/377)).
- docs: fluid 50k bench fits the 600s cap ([#378](https://github.com/lazyxeon/AstraWeave/pull/378)).
- docs: benchmark matrix is 16 shards ([#379](https://github.com/lazyxeon/AstraWeave/pull/379)).
- Deterministic saves, replay apply, and macOS greybox golden ([#381](https://github.com/lazyxeon/AstraWeave/pull/381)).
- Darken cloud shadows with CLOUD_SHADOW_STRENGTH ([#384](https://github.com/lazyxeon/AstraWeave/pull/384)).
- docs: saves, gather plans, and cloud shadow strength ([#386](https://github.com/lazyxeon/AstraWeave/pull/386)).
- Reject weave origins that overflow the millimetre conversion ([#388](https://github.com/lazyxeon/AstraWeave/pull/388)).
- Stop recording the unused Hi-Z pyramid (#269) ([#389](https://github.com/lazyxeon/AstraWeave/pull/389)).
- ci: clear the red badges and the nightly job failures ([#390](https://github.com/lazyxeon/AstraWeave/pull/390)).
- docs: shorten the root README and park loose root files ([#392](https://github.com/lazyxeon/AstraWeave/pull/392)).
- docs: wiring campaign inventory ([#393](https://github.com/lazyxeon/AstraWeave/pull/393)).
- docs: name the cloud-shadow pass and correct the world hash ([#396](https://github.com/lazyxeon/AstraWeave/pull/396)).
- Reject out-of-island weave casts in the mod sandbox ([#397](https://github.com/lazyxeon/AstraWeave/pull/397)).
- docs: record the weave origin island (#388, closes #295) ([#398](https://github.com/lazyxeon/AstraWeave/pull/398)).
- Cap every image read and fix the 1×1 Hi-Z mip (#400) ([#401](https://github.com/lazyxeon/AstraWeave/pull/401)).
- Dispatch indirect GPU cull behind a quality setting (#372) ([#402](https://github.com/lazyxeon/AstraWeave/pull/402)).
- Retire the dormant float destruction manager ([#406](https://github.com/lazyxeon/AstraWeave/pull/406)).
- test: scan every player-facing asset for Veilweaver ([#409](https://github.com/lazyxeon/AstraWeave/pull/409)).
- Correct the removed destruction symbol and the test header ([#414](https://github.com/lazyxeon/AstraWeave/pull/414)).
- Break physics_demo3d boxes with a destruction command ([#415](https://github.com/lazyxeon/AstraWeave/pull/415)).
- Place greybox props from the column forest record ([#417](https://github.com/lazyxeon/AstraWeave/pull/417)).
- Fail the player-facing name scan closed ([#418](https://github.com/lazyxeon/AstraWeave/pull/418)).
- docs: correct the leftovers from the merged reviews ([#419](https://github.com/lazyxeon/AstraWeave/pull/419)).
- Make AGENTS.md the agent constitution ([#420](https://github.com/lazyxeon/AstraWeave/pull/420)).
- test: lock mikktspace tangents on a skewed triangle ([#424](https://github.com/lazyxeon/AstraWeave/pull/424)).
- Turn the root README into a landing page ([#425](https://github.com/lazyxeon/AstraWeave/pull/425)).
- ci: split Windows Test full under the 60-minute cap ([#441](https://github.com/lazyxeon/AstraWeave/pull/441)).
- test(terrain): keep the stitching diagnostic under the slow-timeout ([#442](https://github.com/lazyxeon/AstraWeave/pull/442)).
- ci: stop audit and prose jobs dying in rust setup ([#443](https://github.com/lazyxeon/AstraWeave/pull/443)).
- test(render): hook instance_debug_flags to one Linux thread ([#445](https://github.com/lazyxeon/AstraWeave/pull/445)).
- ci: give the replay hash job time to finish a cold compile ([#447](https://github.com/lazyxeon/AstraWeave/pull/447)).
- Tighten aw_doc_lint frame figures and measurement sentences ([#460](https://github.com/lazyxeon/AstraWeave/pull/460)).
- Use the repo logo as the README hero ([#461](https://github.com/lazyxeon/AstraWeave/pull/461)).
- ci(render): keep the heaptrack workflow parseable ([#462](https://github.com/lazyxeon/AstraWeave/pull/462)).
- ci(bench): install PowerShell from the pinned 7.5.4 tarball ([#466](https://github.com/lazyxeon/AstraWeave/pull/466)).
- ci(windows): run CARGO_BIN_EXE tests on the Test full archive job ([#467](https://github.com/lazyxeon/AstraWeave/pull/467)).
- docs(ci): profile Actions wall clock and rank bottlenecks ([#474](https://github.com/lazyxeon/AstraWeave/pull/474)).
- ci: scope heavy PR workflows and fail safe on workflow edits ([#475](https://github.com/lazyxeon/AstraWeave/pull/475)).
- ci(aravath): install audit, deny, and cyclonedx from pinned builds ([#476](https://github.com/lazyxeon/AstraWeave/pull/476)).
- ci: archive Linux nextest once and run three partitions ([#477](https://github.com/lazyxeon/AstraWeave/pull/477)).
- ci: timeout the nightly terrain bit-lock jobs ([#478](https://github.com/lazyxeon/AstraWeave/pull/478)).
- ci: sccache read-only on pull requests, stable Linux CPU pin ([#480](https://github.com/lazyxeon/AstraWeave/pull/480)).
- ci: drop debuginfo on the replay hash builds ([#481](https://github.com/lazyxeon/AstraWeave/pull/481)).
- Note the landed Aravath tool install and terrain bit-lock timeout ([#482](https://github.com/lazyxeon/AstraWeave/pull/482)).
- Match doc-lint phrases on the pulldown-cmark render ([#484](https://github.com/lazyxeon/AstraWeave/pull/484)).
- test(net): assert bincode LimitExceeded at the cap ([#486](https://github.com/lazyxeon/AstraWeave/pull/486)).
- Record the 4 MiB bincode cap in the audit and the PoC helper ([#488](https://github.com/lazyxeon/AstraWeave/pull/488)).
- Align SEC-NET-3 with the 4 MiB cap ([#489](https://github.com/lazyxeon/AstraWeave/pull/489)).
- Record the landed caps in the security audit ([#490](https://github.com/lazyxeon/AstraWeave/pull/490)).
- test(aw-save): lock the 16 MiB uncompressed LZ4 cap ([#491](https://github.com/lazyxeon/AstraWeave/pull/491)).
- Mark the landed security findings closed on main ([#492](https://github.com/lazyxeon/AstraWeave/pull/492)).
- Say ServerConfig::default installs the dev key on every build ([#493](https://github.com/lazyxeon/AstraWeave/pull/493)).
- Correct the remaining security-audit cites ([#494](https://github.com/lazyxeon/AstraWeave/pull/494)).
- Refresh the net-ecs line cites against main ([#495](https://github.com/lazyxeon/AstraWeave/pull/495)).
- ci: fetch the merge parents in Detect affected crates ([#498](https://github.com/lazyxeon/AstraWeave/pull/498)).
- ci: stop the nightly benchmark cron ([#519](https://github.com/lazyxeon/AstraWeave/pull/519)).
- docs(ai): name the silent Qwen thinking read timeout ([#520](https://github.com/lazyxeon/AstraWeave/pull/520)).
- Record that workflow edits set full=true ([#522](https://github.com/lazyxeon/AstraWeave/pull/522)).

- fix(ci): link the SDK CMake example and install editor native libs ([`05c2bff0e`](https://github.com/lazyxeon/AstraWeave/commit/05c2bff0e)).
- fix(ci): SDK harness load path, save-slot isolation, libudev ([`e7db0dba4`](https://github.com/lazyxeon/AstraWeave/commit/e7db0dba4)).
- fix(ci): C header language, glTF import, and schedule expects ([`b234452bf`](https://github.com/lazyxeon/AstraWeave/commit/b234452bf)).
- fix(ci): null audio backend, bit-stable caustics, and clippy ([`e9b4c3419`](https://github.com/lazyxeon/AstraWeave/commit/e9b4c3419)).
- feat(sim): seed craft, harvest, weaving, and solver draws ([`a570aaf4c`](https://github.com/lazyxeon/AstraWeave/commit/a570aaf4c)).
- feat(ai): plan every companion on the tick ([`8ae54ce84`](https://github.com/lazyxeon/AstraWeave/commit/8ae54ce84)).
- feat(perception): order team, enemy, and obstacle snapshots ([`e9b6212e6`](https://github.com/lazyxeon/AstraWeave/commit/e9b6212e6)).
- feat(determinism): add cross-OS replay-hash CI for planner_pure ([`8be0b00be`](https://github.com/lazyxeon/AstraWeave/commit/8be0b00be)).
- docs: add engine production roadmap with a determinism gate ([`de5832b50`](https://github.com/lazyxeon/AstraWeave/commit/de5832b50)).

#### Fixed
- fix: cap hostile parsers and bind servers to loopback ([#290](https://github.com/lazyxeon/AstraWeave/pull/290)).
- fix: bound model text, imports, admin auth, and socket frames ([#312](https://github.com/lazyxeon/AstraWeave/pull/312)).
- fix: cap remaining image decodes and jail the last model and glTF holes ([#316](https://github.com/lazyxeon/AstraWeave/pull/316)).
- fix: keep live models, GOAP, and gravity off the sim tick ([#331](https://github.com/lazyxeon/AstraWeave/pull/331)).
- fix: close save, quest, checkpoint, and replay determinism holes ([#332](https://github.com/lazyxeon/AstraWeave/pull/332)).
- Fix workspace test reds without skipping them ([#333](https://github.com/lazyxeon/AstraWeave/pull/333)).
- Fix editor/engine parity and the Windows test flakes ([#340](https://github.com/lazyxeon/AstraWeave/pull/340)).
- fix: nest dialogue prompt keys and lock each conversation ([#342](https://github.com/lazyxeon/AstraWeave/pull/342)).
- Fix brush restore, border digs, and cached sword damage ([#343](https://github.com/lazyxeon/AstraWeave/pull/343)).
- fix: drop zbus 5 from the egui and file-dialog graph ([#365](https://github.com/lazyxeon/AstraWeave/pull/365)).
- Fix shard 7 raycast bench exit 101 ([#367](https://github.com/lazyxeon/AstraWeave/pull/367)).
- fix: drop unsafe-libyaml by parsing traces with yaml_serde (#311) ([#380](https://github.com/lazyxeon/AstraWeave/pull/380)).
- fix(ai): drop the arbiter and both GOAP trees ([#382](https://github.com/lazyxeon/AstraWeave/pull/382)).
- fix: drop number_prefix by upgrading indicatif to 0.18 (#307) ([#385](https://github.com/lazyxeon/AstraWeave/pull/385)).
- fix: drop atomic-polyfill from postcard (#320) ([#387](https://github.com/lazyxeon/AstraWeave/pull/387)).
- fix: remove Veilweaver from player-facing asset text ([#394](https://github.com/lazyxeon/AstraWeave/pull/394)).
- fix(render): Bruneton atmosphere stays off until measured ([#395](https://github.com/lazyxeon/AstraWeave/pull/395)).
- fix: drop nalgebra 0.26 from mikktspace tangents (#306) ([#407](https://github.com/lazyxeon/AstraWeave/pull/407)).
- fix(render): close the #402 GPU-cull review ([#421](https://github.com/lazyxeon/AstraWeave/pull/421)).
- fix: bump wayland-scanner onto quick-xml 0.41 (#308) ([#426](https://github.com/lazyxeon/AstraWeave/pull/426)).
- fix: finish GPU work before the software adapter gives up ([#427](https://github.com/lazyxeon/AstraWeave/pull/427)).
- fix: reject userinfo in loopback model URLs ([#429](https://github.com/lazyxeon/AstraWeave/pull/429)).
- fix(render): refuse Bruneton when fog and shafts are both off ([#430](https://github.com/lazyxeon/AstraWeave/pull/430)).
- fix: close the GPU review gaps left after #427 ([#432](https://github.com/lazyxeon/AstraWeave/pull/432)).
- fix(dialogue): keep the player-facing scan out of library clippy ([#435](https://github.com/lazyxeon/AstraWeave/pull/435)).
- fix(render): reserve a skinning slot without expect ([#436](https://github.com/lazyxeon/AstraWeave/pull/436)).
- fix(render): stop the Mesa 26 lib-test SIGSEGV (#434) ([#438](https://github.com/lazyxeon/AstraWeave/pull/438)).
- fix(render): record a skip when macOS has no GPU adapter ([#439](https://github.com/lazyxeon/AstraWeave/pull/439)).
- fix(render): keep draw_into inside the alloc floor when RUST_BACKTRACE is set ([#440](https://github.com/lazyxeon/AstraWeave/pull/440)).
- fix: cap model body reads at 64 KiB ([#448](https://github.com/lazyxeon/AstraWeave/pull/448)).
- fix: budget raw NDJSON separately from extracted text ([#449](https://github.com/lazyxeon/AstraWeave/pull/449)).
- fix(llm): keep slow Ollama chunks on macOS (#453) ([#459](https://github.com/lazyxeon/AstraWeave/pull/459)).
- fix(deps): bump zstd to 0.14.3 (RUSTSEC-2026-0391) ([#463](https://github.com/lazyxeon/AstraWeave/pull/463)).
- fix(input): give save tests private temp directories ([#468](https://github.com/lazyxeon/AstraWeave/pull/468)).
- fix(asset): reject a GLB length shorter than the header ([#469](https://github.com/lazyxeon/AstraWeave/pull/469)).
- fix(net): cap bincode decode at the 4 MiB LZ4 bound ([#470](https://github.com/lazyxeon/AstraWeave/pull/470)).
- fix(ai): import GatherGoal in the CAiController example ([#471](https://github.com/lazyxeon/AstraWeave/pull/471)).
- fix(render): size terrain height_blend to 32 layers ([#472](https://github.com/lazyxeon/AstraWeave/pull/472)).
- fix(llm): keep slow Ollama chunks when the runner wakes late ([#473](https://github.com/lazyxeon/AstraWeave/pull/473)).
- fix(net): drop ghost players when the join send fails ([#523](https://github.com/lazyxeon/AstraWeave/pull/523)).
- fix(ecs): do not allocate a zero-sized BlobVec ([#527](https://github.com/lazyxeon/AstraWeave/pull/527)).
- fix(aw_doc_lint): byte-safe retired-title look-behind ([#529](https://github.com/lazyxeon/AstraWeave/pull/529)).

#### Security
- chore(deps): bump tokio to 1.53.3 (RUSTSEC-2026-0386) ([#458](https://github.com/lazyxeon/AstraWeave/pull/458)).
- Route security reports to GitHub private vulnerability reporting ([#510](https://github.com/lazyxeon/AstraWeave/pull/510)).

### 2026-08

#### Changed
- feat(l3d): the c3 fix ladder — rung 2 halves the pop for less than the shipped cost, and still misses the bar ([`a3fe3ecdc`](https://github.com/lazyxeon/AstraWeave/commit/a3fe3ecdc)).
- docs(l3c-res): close-out record — engine-wide win confirmed, HEALTH-lane handoff, levers held ([`724f13de0`](https://github.com/lazyxeon/AstraWeave/commit/724f13de0)).
- feat(l3c-res): live-c2 ships on budget — but the stepping is c3's, not c2's ([`ed35b3b35`](https://github.com/lazyxeon/AstraWeave/commit/ed35b3b35)).
- feat(l3c): seam hypothesis CONFIRMED; 4th cascade built — and it triples far-field stepping ([`154d723aa`](https://github.com/lazyxeon/AstraWeave/commit/154d723aa)).

### 2026-07

#### Changed
- Campaign/roadmap ([#198](https://github.com/lazyxeon/AstraWeave/pull/198)).

- 2 `assets(ad4)` commits on `main`, newest "assets(ad4): re-point live consumers to derived_1k sample slots" ([`06780433d`](https://github.com/lazyxeon/AstraWeave/commit/06780433d)) through oldest "assets(ad4): cook 36 1K sample derivatives + re-cook 9 traced families in place" ([`333f38472`](https://github.com/lazyxeon/AstraWeave/commit/333f38472)).
- assets(ad5): manifest schema v2 + packlists + generated ignore surfaces + CI guard ([`4f3d8c365`](https://github.com/lazyxeon/AstraWeave/commit/4f3d8c365)).
- 2 `chore` commits on `main`, newest "chore: untrack .recent_files.json (editor-local state, swept in by git add -A)" ([`d6f53c00e`](https://github.com/lazyxeon/AstraWeave/commit/d6f53c00e)) through oldest "chore: refresh tracked local prefs — camera fov→fovy schema, Claude permission entry" ([`ac99a0608`](https://github.com/lazyxeon/AstraWeave/commit/ac99a0608)).
- chore(ad5): Cargo.lock (fs2 dep) + AD4A doc lint formatting ([`5e27c69d1`](https://github.com/lazyxeon/AstraWeave/commit/5e27c69d1)).
- chore(ad6): LFS decommission terminal commit ([`dee60b294`](https://github.com/lazyxeon/AstraWeave/commit/dee60b294)).
- chore(hygiene): delete 36 orphan baked/*.ktx2.meta.json sidecars (close-out §4 item 11) ([`8c47d918d`](https://github.com/lazyxeon/AstraWeave/commit/8c47d918d)).
- chore(showcase): remove unified_showcase from the workspace (director-ratified 2026-07-19) ([`ef9ab9c44`](https://github.com/lazyxeon/AstraWeave/commit/ef9ab9c44)).
- 5 `ci` commits on `main`, newest "ci: separate demo build from smoke timeout" ([`273ded3a9`](https://github.com/lazyxeon/AstraWeave/commit/273ded3a9)) through oldest "ci: retire workflow exceptions" ([`1df55f19f`](https://github.com/lazyxeon/AstraWeave/commit/1df55f19f)).
- ci(ad6): remove orphaned empty 'with:' block in material-validation ([`360a4f125`](https://github.com/lazyxeon/AstraWeave/commit/360a4f125)).
- 3 `ci(benchmarks)` commits on `main`, newest "ci(benchmarks): clarify full outcome accounting" ([`1652ab356`](https://github.com/lazyxeon/AstraWeave/commit/1652ab356)) through oldest "ci(benchmarks): repair recursive collection and honest aggregation" ([`e3689ac85`](https://github.com/lazyxeon/AstraWeave/commit/e3689ac85)).
- diag(l2): STOP at Phase 0 -- irradiance convolution face-blind; premise falsified ([`4c139460d`](https://github.com/lazyxeon/AstraWeave/commit/4c139460d)).
- diag(l3b): the stepping is not the cache — four refutations, two rejected fixes, one retraction, STOP ([`f11e06536`](https://github.com/lazyxeon/AstraWeave/commit/f11e06536)).
- diag(t2d cont): director sign REPRODUCED at altitude; mip0-boost hypothesis falsified 3 ways; STOP for ED-2 ([`38c293106`](https://github.com/lazyxeon/AstraWeave/commit/38c293106)).
- 2 `diag(t2d)` commits on `main`, newest "diag(t2d): boundary IDENTIFIED as compute_material_lod's LOD1|2 threshold; 3 inert-UI defects" ([`d4c4479d4`](https://github.com/lazyxeon/AstraWeave/commit/d4c4479d4)) through oldest "diag(t2d): camera-light defect — camera dependence CONFIRMED, boundary NOT reproduced; STOP before fixing" ([`215155237`](https://github.com/lazyxeon/AstraWeave/commit/215155237)).
- diag(t2f): lighting recon -- terrain lit by one unshadowed sun + 4% wash; options costed ([`1d8e1c792`](https://github.com/lazyxeon/AstraWeave/commit/1d8e1c792)).
- docs(ad0): curated sample-set proposal — 154 assets, ~93.7 MB budget ([`a12f7d958`](https://github.com/lazyxeon/AstraWeave/commit/a12f7d958)).
- docs(ad1): asset provenance — third-party license manifest + generation evidence ([`1450ceb32`](https://github.com/lazyxeon/AstraWeave/commit/1450ceb32)).
- docs(ad1b): trace pack-bucket remainder — pine_forest/textures/models/tests provenance ([`5c88e12ff`](https://github.com/lazyxeon/AstraWeave/commit/5c88e12ff)).
- docs(ad1c): residual trace (84 TRACED/15 QUAR) + disposition ratification — gate bucket → 0, no provisional dispositions remain ([`4659400d1`](https://github.com/lazyxeon/AstraWeave/commit/4659400d1)).
- docs(ad2): Git LFS removal plan — sole-authority proof + ecosystem cleanup list ([`312a3985c`](https://github.com/lazyxeon/AstraWeave/commit/312a3985c)).
- docs(ad3): asset-pack release assets-v1 — 18 zips built, loop-verified local+live, uploaded, published ([`b03a6e355`](https://github.com/lazyxeon/AstraWeave/commit/b03a6e355)).
- 2 `docs(ad4)` commits on `main`, newest "docs(ad4): cut materials-src pack (live on assets-v1) + close-out" ([`0fdfd2b93`](https://github.com/lazyxeon/AstraWeave/commit/0fdfd2b93)) through oldest "docs(ad4): C7 re-acquisition provenance + restore clobbered ATTRIBUTION evidence" ([`230899962`](https://github.com/lazyxeon/AstraWeave/commit/230899962)).
- docs(ad4a): render-gate diagnosis — causal chains, Phase-3 baseline, fixes, options ([`6c21427a3`](https://github.com/lazyxeon/AstraWeave/commit/6c21427a3)).
- docs(ad5): reconnection outcome — installer live-verified, Tier-2 rehearsal complete ([`703557ade`](https://github.com/lazyxeon/AstraWeave/commit/703557ade)).
- docs(ad5a): consumer-hygiene outcome note — 13-ref ledger, launch evidence, 7-entry paintable table, GO verdict ([`1b3f9ae84`](https://github.com/lazyxeon/AstraWeave/commit/1b3f9ae84)).
- docs(ad5a-4): asset_manifest ice provenance correction + ARM-order annotation ([`0543e6478`](https://github.com/lazyxeon/AstraWeave/commit/0543e6478)).
- 3 `docs(ad6)` commits on `main`, newest "docs(ad6): close-out §4 item-1 amendment per CI_WORKSHOP §4 — director rider" ([`1cc824082`](https://github.com/lazyxeon/AstraWeave/commit/1cc824082)) through oldest "docs(ad6): extract founding concept doc from Windows-invalid historical path" ([`15e27ff89`](https://github.com/lazyxeon/AstraWeave/commit/15e27ff89)).
- docs(ad6r): rewrite recon — rehearsed end-to-end, all gate inputs measured ([`62b158156`](https://github.com/lazyxeon/AstraWeave/commit/62b158156)).
- docs(audit): record workspace lint inventory ([`ffc2b9494`](https://github.com/lazyxeon/AstraWeave/commit/ffc2b9494)).
- docs(audits): asset ground-truth audit — report, 91k-row registry, imported-set disposition ([`0d2ccba6f`](https://github.com/lazyxeon/AstraWeave/commit/0d2ccba6f)).
- 2 `docs(board)` commits on `main`, newest "docs(board): append post-push board read — toolchain-workflow baseline gap + item-12 escalation trigger" ([`5e59136ac`](https://github.com/lazyxeon/AstraWeave/commit/5e59136ac)) through oldest "docs(board): board-state note 2026-07-19 — director dispositions + w0-pre-deprecation tag finding" ([`6fa70c740`](https://github.com/lazyxeon/AstraWeave/commit/6fa70c740)).
- docs(ci): add quests benchmark timeout handoff ([`06e612756`](https://github.com/lazyxeon/AstraWeave/commit/06e612756)).
- docs(ci-w): record workshop outcome ([`3d4988547`](https://github.com/lazyxeon/AstraWeave/commit/3d4988547)).
- docs(e3-pf): terrain campaign resumption pre-flight — the E3 build already landed (d506658d8); reconstruction, drift audit, SP5 disposition, mapping ratification package, T-series plan ([`8232b150b`](https://github.com/lazyxeon/AstraWeave/commit/8232b150b)).
- docs(e3-recon): E3-terrain generation-pipeline wiring recon — consumption-collapse, not bypass (zero code change) ([`187b8a8d6`](https://github.com/lazyxeon/AstraWeave/commit/187b8a8d6)).
- docs(editor): close multi-tool sub-phase 5 ([`379f1e3fa`](https://github.com/lazyxeon/AstraWeave/commit/379f1e3fa)).
- docs(health): record H1 mechanical lint outcome ([`770ce1274`](https://github.com/lazyxeon/AstraWeave/commit/770ce1274)).
- docs(lint): allowlist seven cited audit occurrences ([`6dad06ccb`](https://github.com/lazyxeon/AstraWeave/commit/6dad06ccb)).
- 2 `docs(t0)` commits on `main`, newest "docs(t0): addendum delta — 1df55f19f added astraweave-terrain to the Clippy P1 warn matrix" ([`7e52c290c`](https://github.com/lazyxeon/AstraWeave/commit/7e52c290c)) through oldest "docs(t0): ratification record + build outcome note + close-out item-1 correction + w0 recovery-path sweep" ([`d3806e300`](https://github.com/lazyxeon/AstraWeave/commit/d3806e300)).
- 4 `docs(t2a)` commits on `main`, newest "docs(t2a): sweep landed — 2440/64; the +1 failure is Phase 3's, named and classified" ([`bf57b5f1d`](https://github.com/lazyxeon/AstraWeave/commit/bf57b5f1d)) through oldest "docs(t2a): outcome note + terrain v1.3 / terrain_materials v1.4 / render v1.11" ([`fd667536d`](https://github.com/lazyxeon/AstraWeave/commit/fd667536d)).
- docs(trace-sync): T.0 — pay the E3 build's four-trace debt (terrain v1.2, render v1.10, aw_editor v1.5, terrain_materials v1.2) ([`475f837ea`](https://github.com/lazyxeon/AstraWeave/commit/475f837ea)).
- 3 `docs(tw1)` commits on `main`, newest "docs(tw1): fix commit-hash citations after pull-rebase rewrote the beat commits" ([`5059acc03`](https://github.com/lazyxeon/AstraWeave/commit/5059acc03)) through oldest "docs(tw1): ratification record — T.W-R gate answers + beat split + T.1 rung-3 PASS" ([`f398a3778`](https://github.com/lazyxeon/AstraWeave/commit/f398a3778)).
- docs(tw1a): outcome note + water trace v1.3 (horizon shell, fade, style params) ([`fc4964538`](https://github.com/lazyxeon/AstraWeave/commit/fc4964538)).
- docs(tw2): outcome note + water v1.4 / aw_editor v1.7 trace bumps ([`91d170ef2`](https://github.com/lazyxeon/AstraWeave/commit/91d170ef2)).
- docs(twr): water recon — state-of-water at HEAD, integration map, paint-semantics options (design gate) ([`09c589a5d`](https://github.com/lazyxeon/AstraWeave/commit/09c589a5d)).
- feat(ad5): transactional original-path pack installer (xtask) + rails ([`89e6ba78b`](https://github.com/lazyxeon/AstraWeave/commit/89e6ba78b)).
- feat(ad5a-3): name-based paint-palette remap (AD.4.A 5a Option C, ratified) ([`ae9b98ef3`](https://github.com/lazyxeon/AstraWeave/commit/ae9b98ef3)).
- feat(ed2): editor visual-verification tooling — camera pin/restore + viewport capture ([`03f9d9a2d`](https://github.com/lazyxeon/AstraWeave/commit/03f9d9a2d)).
- feat(ed3): no inert controls -- shading modes live, Normals/UVs built, amplitude lever real ([`09ea417af`](https://github.com/lazyxeon/AstraWeave/commit/09ea417af)).
- feat(editor): AstraWeave branded theme as default — palette single-source, branded chrome, splash video re-enabled ([`7d51c0267`](https://github.com/lazyxeon/AstraWeave/commit/7d51c0267)).
- feat(l1): lighting calibration & honesty -- race fixed, exposure live, fictions deleted; 0-pixel neutral ([`2e938ff9d`](https://github.com/lazyxeon/AstraWeave/commit/2e938ff9d)).
- feat(l2 calibration): director-ratified A+B+E -- sun-clamped bake, fixed intensity 1.5, worker-thread bake ([`fcef1ba60`](https://github.com/lazyxeon/AstraWeave/commit/fcef1ba60)).
- feat(l2): IBL wired into editor terrain -- bake at init, group(3), split-sum x AO; calibration STOPPED for ratification ([`5b78fdd78`](https://github.com/lazyxeon/AstraWeave/commit/5b78fdd78)).
- feat(l3): terrain casts + receives the CSM — cascades finally have their terrain consumer ([`9fefac61e`](https://github.com/lazyxeon/AstraWeave/commit/9fefac61e)).
- feat(t1): distinct beach material — coast_sand_01 cooked into biomes slot 6 (ratification §2 row-6 amendment) ([`1f0b48b5e`](https://github.com/lazyxeon/AstraWeave/commit/1f0b48b5e)).
- feat(t2a): pinned-station A/B harness + material channel-stats instrument ([`50f5d6f1b`](https://github.com/lazyxeon/AstraWeave/commit/50f5d6f1b)).
- feat(t2c): real PBR materials for the three synthetic biome slots (0 grassland, 1 desert, 4 tundra) ([`c2dbf8400`](https://github.com/lazyxeon/AstraWeave/commit/c2dbf8400)).
- feat(t2d.f): retire the material-LOD tiers -- boundary + dithering measured gone ([`aca627436`](https://github.com/lazyxeon/AstraWeave/commit/aca627436)).
- feat(terrain): E3 real build — climate-driven splines, per-archetype landform, coherent biomes, PBR terrain texturing ([`d506658d8`](https://github.com/lazyxeon/AstraWeave/commit/d506658d8)).
- feat(tw1): surface the water system in the editor — format fix, E3 gate, sea-level tie, hygiene (TW1 ratification) ([`88c6ba588`](https://github.com/lazyxeon/AstraWeave/commit/88c6ba588)).
- feat(tw1a): world-scale water presentation — horizon shell, distance fade, per-style wave params ([`49f1e3396`](https://github.com/lazyxeon/AstraWeave/commit/49f1e3396)).
- feat(tw2): paintable water — Carve Water brush + WaterVolume entities ([`1e3d8f692`](https://github.com/lazyxeon/AstraWeave/commit/1e3d8f692)).
- feat(xtask): AD.2 `cargo xtask fetch-assets` — release-pack restore with sha256 gate ([`916a0d9a0`](https://github.com/lazyxeon/AstraWeave/commit/916a0d9a0)).
- fix(ad4a): repair 8 derived_1k _mra maps — ARM-order inversion (D1) + 16-bit collapse (D2) ([`09b79647b`](https://github.com/lazyxeon/AstraWeave/commit/09b79647b)).
- 2 `fix(ad5)` commits on `main`, newest "fix(ad5): scope the verify-assets gate to installer-owned references" ([`a2955685d`](https://github.com/lazyxeon/AstraWeave/commit/a2955685d)) through oldest "fix(ad5): cross-pack prune guard (H1) + assets_src ignore-block coverage (M2)" ([`4e0aec91b`](https://github.com/lazyxeon/AstraWeave/commit/4e0aec91b)).
- fix(ad5a-1): re-point biomes forest slot to shipped derived_1k tree_leaves ([`21bc53333`](https://github.com/lazyxeon/AstraWeave/commit/21bc53333)).
- fix(ad5a-2): unified_showcase degrades on missing textures instead of crashing ([`75210705c`](https://github.com/lazyxeon/AstraWeave/commit/75210705c)).
- fix(adaptive_boss,hello_companion,slice): clear mechanical lints ([`9ac074dbd`](https://github.com/lazyxeon/AstraWeave/commit/9ac074dbd)).
- fix(ai): mark fast executor as deferred wiring ([`ae9a1b280`](https://github.com/lazyxeon/AstraWeave/commit/ae9a1b280)).
- fix(ai,ecs,rag,embeddings): clear mechanical clippy findings ([`d7cc8960a`](https://github.com/lazyxeon/AstraWeave/commit/d7cc8960a)).
- fix(asset,scene,gameplay,dialogue,weaving,net): clear mechanical lints ([`10ca7d522`](https://github.com/lazyxeon/AstraWeave/commit/10ca7d522)).
- fix(assets): merge attribution entries without clobbering ([`5778fd17b`](https://github.com/lazyxeon/AstraWeave/commit/5778fd17b)).
- fix(ed1): editor first-launch "hang" — investigation outcome + startup discriminator tracing ([`47c857acc`](https://github.com/lazyxeon/AstraWeave/commit/47c857acc)).
- 3 `fix(editor)` commits on `main`, newest "fix(editor): startup maximize fires reliably — gate on genuine visibility + authoritative ShowWindow" ([`9607620e8`](https://github.com/lazyxeon/AstraWeave/commit/9607620e8)) through oldest "fix(editor): grid shader writes real frag_depth — entities no longer hidden by the ground fill" ([`21407ce4e`](https://github.com/lazyxeon/AstraWeave/commit/21407ce4e)).
- fix(l2 ibl): face-correct irradiance convolution; un-flip all bake-pass writes ([`b1295b93f`](https://github.com/lazyxeon/AstraWeave/commit/b1295b93f)).
- fix(l3a): the shadow boundary was the camera, not the world — survey cascade + motion-verified ([`cc17a99da`](https://github.com/lazyxeon/AstraWeave/commit/cc17a99da)).
- 2 `fix(t2a)` commits on `main`, newest "fix(t2a): complete the aux repair across all traced-9 families" ([`70eabe2df`](https://github.com/lazyxeon/AstraWeave/commit/70eabe2df)) through oldest "fix(t2a): repair the biome pack's corrupt roughness/AO data (Phase 1)" ([`52081105f`](https://github.com/lazyxeon/AstraWeave/commit/52081105f)).
- fix(terrain): retire seven mechanical clippy findings blocking Code Quality ([`89fbe97eb`](https://github.com/lazyxeon/AstraWeave/commit/89fbe97eb)).
- merge: H.1 mechanical lint batch ([`8645d5ee2`](https://github.com/lazyxeon/AstraWeave/commit/8645d5ee2)).
- refactor(terrain): retire the crate's six production expect()s (T.2a Phase 4) ([`0beec9b62`](https://github.com/lazyxeon/AstraWeave/commit/0beec9b62)).
- style: repo-wide cargo fmt under the pinned 1.89.0 toolchain (retires the CI formatting exceptions) ([`1ef81c239`](https://github.com/lazyxeon/AstraWeave/commit/1ef81c239)).
- style(editor): rustfmt churn across aw_editor sources ([`bc284bb65`](https://github.com/lazyxeon/AstraWeave/commit/bc284bb65)).
- test(editor): guard regional paint pointer consumption ([`2e92b8a3b`](https://github.com/lazyxeon/AstraWeave/commit/2e92b8a3b)).
- tools(ad4): 1K-derivative material cook (cook_1k.py + contract test) ([`6fd0330e0`](https://github.com/lazyxeon/AstraWeave/commit/6fd0330e0)).
- 2 `tune(t2a)` commits on `main`, newest "tune(t2a): hex-tile weight sharpening pow 4.0 -> 2.0 (Phase 2.2, isolated)" ([`bcffe81dd`](https://github.com/lazyxeon/AstraWeave/commit/bcffe81dd)) through oldest "tune(t2a): NORMAL_XY_STRENGTH 1.8 -> 1.4 (Phase 2.1, isolated)" ([`80780567e`](https://github.com/lazyxeon/AstraWeave/commit/80780567e)).
- tune(terrain): TUNDRA_MAX_TEMP_C 0.0 -> -5.0, boreal taiga threading (T.2a Phase 3) ([`b3051e71b`](https://github.com/lazyxeon/AstraWeave/commit/b3051e71b)).

### 2026-06

#### Changed
- Campaign/fluids f1 ([#192](https://github.com/lazyxeon/AstraWeave/pull/192)).
- Campaign/water successor ([#194](https://github.com/lazyxeon/AstraWeave/pull/194)).
- Campaign/doc truth resume ([#197](https://github.com/lazyxeon/AstraWeave/pull/197)).

- docs: Update CLAIMS_REGISTRY and CLOSED_VOCABULARY_ALLOWLIST; add D.2.A Execution Report ([`5bd2d7c15`](https://github.com/lazyxeon/AstraWeave/commit/5bd2d7c15)).
- docs(architecture): trace campaign — 13 new subsystem traces + water/fluids updates + map refresh ([`5611de364`](https://github.com/lazyxeon/AstraWeave/commit/5611de364)).
- docs(architecture/_meta): add TRACE_SYNC_PROPOSAL — deterministic trace→registry/map sync ([`ed195223e`](https://github.com/lazyxeon/AstraWeave/commit/ed195223e)).
- docs(e3-recon): M2 E3 terrain test-surface recon — 8 is a ~45-across-13 undercount (zero code change) ([`8405e9263`](https://github.com/lazyxeon/AstraWeave/commit/8405e9263)).
- docs(e3c): dissolve E3.c — scatter "real-defect" was a misdiagnosis, reclassify to test-rot (zero code change) ([`39b85ba8e`](https://github.com/lazyxeon/AstraWeave/commit/39b85ba8e)).
- docs(e4): close E4 via relabel-and-defer — asset-pipeline honest, cook deferred post-v1.0 ([`967575a77`](https://github.com/lazyxeon/AstraWeave/commit/967575a77)).
- docs(e4-recon): M2 E4 cook-path recon — relabel-and-defer proposed (zero code change) ([`15df1d28d`](https://github.com/lazyxeon/AstraWeave/commit/15df1d28d)).
- docs(fluids trace): F.3 revision — voxel grid behind WaterQuery (v1.4) ([`6abfc320a`](https://github.com/lazyxeon/AstraWeave/commit/6abfc320a)).
- docs(fluids-integration): F.3.S plan-of-record update (decline A->C) ([`72b80c208`](https://github.com/lazyxeon/AstraWeave/commit/72b80c208)).
- 7 `docs(roadmap)` commits on `main`, newest "docs(roadmap): M1.3 — sync A2/A5 criteria to wired reality + honest AI-pillar scope" ([`4eddd9651`](https://github.com/lazyxeon/AstraWeave/commit/4eddd9651)) through oldest "docs(roadmap): R.0.A — roadmap recon (verification methodology + topology + pilot)" ([`a430937c9`](https://github.com/lazyxeon/AstraWeave/commit/a430937c9)).
- docs(trace-honesty): M1 close — make traces match verified reality (zero code change) ([`e98487a55`](https://github.com/lazyxeon/AstraWeave/commit/e98487a55)).
- Editor-Multi-Tool-Architecture.Sub-phase-4.A: dispatcher mechanics gap-fill tests (reuse MockActiveTool fixture) ([`d015b6e11`](https://github.com/lazyxeon/AstraWeave/commit/d015b6e11)).
- Editor-Multi-Tool-Architecture.Sub-phase-4.B: lifecycle-ORDER test + EventDisposition forward-compat tripwire + OrderRecordingTool fixture ([`6b00d8928`](https://github.com/lazyxeon/AstraWeave/commit/6b00d8928)).
- Editor-Multi-Tool-Architecture.Sub-phase-4.C: closeout — Status header + §11 SP4 entry COMPLETE + §12 deviation entry ([`2e522767d`](https://github.com/lazyxeon/AstraWeave/commit/2e522767d)).
- Editor-Multi-Tool-Architecture.Sub-phase-5.A: live the dispatcher Y=0 projection path (part A) ([`2f6f853a5`](https://github.com/lazyxeon/AstraWeave/commit/2f6f853a5)).
- Editor-Multi-Tool-Architecture.Sub-phase-5.B: RegionalArchetypePanel ActiveTool + registration + paint command/undo (parts B-E) ([`9bae816e3`](https://github.com/lazyxeon/AstraWeave/commit/9bae816e3)).
- Engine-Health-Audit.2026-06-10: full health audit + workspace-wide doc reconciliation ([`1ca541aaa`](https://github.com/lazyxeon/AstraWeave/commit/1ca541aaa)).
- feat: integrate astraweave-fluids dependency into astraweave-water package docs: update Fluids Integration Campaign Plan to reflect F.3 completion and next steps ([`3645282f7`](https://github.com/lazyxeon/AstraWeave/commit/3645282f7)).
- 2 `feat(tools)` commits on `main`, newest "feat(tools): aw_trace_sync v1.1 — map status sync + runtime_edges" ([`9113098f8`](https://github.com/lazyxeon/AstraWeave/commit/9113098f8)) through oldest "feat(tools): aw_trace_sync — deterministic trace registry/map synchronizer (v1)" ([`b15e48c7c`](https://github.com/lazyxeon/AstraWeave/commit/b15e48c7c)).
- fix(llm_integration): provide DEFAULT_QWEN_INSTRUCT_MODEL in no-feature build ([`7b9412970`](https://github.com/lazyxeon/AstraWeave/commit/7b9412970)).
- fix(physics): M1.2 — capsule half-extent in the ground snap (grounding bug) ([`feea7a02f`](https://github.com/lazyxeon/AstraWeave/commit/feea7a02f)).
- fix(render): M1.5 — guard zero-area triangles in compute_tangents (mikktspace OOB) ([`0f41839bb`](https://github.com/lazyxeon/AstraWeave/commit/0f41839bb)).
- Fluids-Integration.F.2: execution report (phase deliverable) ([`59c7e4e03`](https://github.com/lazyxeon/AstraWeave/commit/59c7e4e03)).
- Fluids-Integration.F.2 WI-1/WI-2: astraweave-water facade crate + WaterQuery ([`110e0b1b8`](https://github.com/lazyxeon/AstraWeave/commit/110e0b1b8)).
- Fluids-Integration.F.2 WI-3: rewire physics buoyancy onto the WaterQuery facade ([`737fd1bb0`](https://github.com/lazyxeon/AstraWeave/commit/737fd1bb0)).
- Fluids-Integration.F.2 WI-3 cleanup + WI-5 bench: retire EnvironmentManager water, record query cost ([`0281edf2c`](https://github.com/lazyxeon/AstraWeave/commit/0281edf2c)).
- Fluids-Integration.F.3 WI-1/6/7: voxel backend behind WaterQuery + F.3 report ([`8a6ebaeb5`](https://github.com/lazyxeon/AstraWeave/commit/8a6ebaeb5)).
- Fluids-Integration.F.3 WI-2/3/4/5: voxel gate flags real, conservation, terrain boundary ([`c698f9513`](https://github.com/lazyxeon/AstraWeave/commit/c698f9513)).
- Fluids-Integration.F.3.S WI-2/WI-3: sparsity benchmark + high-fill dense guard ([`c6d2a277b`](https://github.com/lazyxeon/AstraWeave/commit/c6d2a277b)).
- Fluids-Integration.F.3.S WI-4: budget verdict + benchmark record + trace (v1.5) ([`8523c8eba`](https://github.com/lazyxeon/AstraWeave/commit/8523c8eba)).
- Merge campaign/fluids-f3s: F-series (Fluids Integration) complete ([`71cc26e0e`](https://github.com/lazyxeon/AstraWeave/commit/71cc26e0e)).
- 3 `misc` commits on `main`, newest "Add Fluids Integration Campaign Plan of Record (Path B)" ([`eba1578a5`](https://github.com/lazyxeon/AstraWeave/commit/eba1578a5)) through oldest "add additional Bash and PowerShell commands for cargo operations and web fetching" ([`7f7dd5075`](https://github.com/lazyxeon/AstraWeave/commit/7f7dd5075)).
- Net-Trio-Remediation.W.1: canonical HMAC-SHA256 sign/verify in aw-net-proto; XOR stub deleted ([`3d38948bc`](https://github.com/lazyxeon/AstraWeave/commit/3d38948bc)).
- Net-Trio-Remediation.W.2.a: client signs via canonical HMAC-SHA256 surface ([`0ea2dc02f`](https://github.com/lazyxeon/AstraWeave/commit/0ea2dc02f)).
- Net-Trio-Remediation.W.2.b: server verifies via canonical surface; SignatureFailurePolicy; lib+bin split ([`5a78e6e0f`](https://github.com/lazyxeon/AstraWeave/commit/5a78e6e0f)).
- Net-Trio-Remediation.W.2.b-fix1: graceful unknown-room handling + cleanup-preserving snapshot-error path ([`9d4df10d7`](https://github.com/lazyxeon/AstraWeave/commit/9d4df10d7)).
- Net-Trio-Remediation.W.3.1: Family 1 — authenticated round-trip tests + shared harness ([`29e2145a2`](https://github.com/lazyxeon/AstraWeave/commit/29e2145a2)).
- Net-Trio-Remediation.W.3.2: Family 2 — tampered and malformed packets (reject-and-survive) ([`03907d2c6`](https://github.com/lazyxeon/AstraWeave/commit/03907d2c6)).
- Net-Trio-Remediation.W.3.3: Family 3 — wrong-key and SignatureFailurePolicy behavior ([`a96c86a81`](https://github.com/lazyxeon/AstraWeave/commit/a96c86a81)).
- Net-Trio-Remediation.W.3.3-fmt: rustfmt normalization of family3 client-binary test ([`8b94026ce`](https://github.com/lazyxeon/AstraWeave/commit/8b94026ce)).
- Net-Trio-Remediation.W.3.4: Family 4 — disconnect paths ([`40a3ed00a`](https://github.com/lazyxeon/AstraWeave/commit/40a3ed00a)).
- Net-Trio-Remediation.W.4: docs closeout ([`9f3616647`](https://github.com/lazyxeon/AstraWeave/commit/9f3616647)).
- Net-Trio-Remediation.W.5.1: SigningKey encapsulation + from_hex timing note + RFC 4231 KAT completion ([`25450b09e`](https://github.com/lazyxeon/AstraWeave/commit/25450b09e)).
- Net-Trio-Remediation.W.5.2: TLS signature-path coverage (closes §5 CRITICAL gap) ([`8dd42ee8a`](https://github.com/lazyxeon/AstraWeave/commit/8dd42ee8a)).
- Net-Trio-Remediation.W.5.3: document server->client asymmetric-trust design (closes §5 finding 5) ([`9a913151d`](https://github.com/lazyxeon/AstraWeave/commit/9a913151d)).
- Terrain Asset Quality: asset-hygiene durable findings (27-PNG divergence + polyhaven missing files) ([`e85b67896`](https://github.com/lazyxeon/AstraWeave/commit/e85b67896)).
- Terrain Asset Quality D-core: absolute texture-footprint baseline (budget-anchored verdict) ([`7f3ce989f`](https://github.com/lazyxeon/AstraWeave/commit/7f3ce989f)).
- Terrain Asset Quality E-closeout: campaign closed (outcome doc + two-layer methodology + cleanup-queue/pause-stack hand-off) ([`a40e8734a`](https://github.com/lazyxeon/AstraWeave/commit/a40e8734a)).
- 2 `test(ai)` commits on `main`, newest "test(ai): A2 — pin no-enemy empty-plan in the wired-path determinism test" ([`4c8a74c90`](https://github.com/lazyxeon/AstraWeave/commit/4c8a74c90)) through oldest "test(ai): M1.3-test / A2 — wired-path determinism contract test (A2 met)" ([`39ecd2b77`](https://github.com/lazyxeon/AstraWeave/commit/39ecd2b77)).
- test(physics): pin ~0 settle-velocity in character_controller_stays_on_ground ([`41d82aa6d`](https://github.com/lazyxeon/AstraWeave/commit/41d82aa6d)).
- test(render): M1.4b — clear the last 3 test-rot themes (render -> 0 failures) ([`ea48f7dea`](https://github.com/lazyxeon/AstraWeave/commit/ea48f7dea)).
- test(render,blend,editor): M1.4a — update stale test-rot assertions to canonical values ([`22869d8e7`](https://github.com/lazyxeon/AstraWeave/commit/22869d8e7)).
- test(terrain): E3.a-1 — amplitude-independent test-rot batch (13 tests, test-side only) ([`500484126`](https://github.com/lazyxeon/AstraWeave/commit/500484126)).
- Unified Camera C.7.C: CameraKeyframe retirement + roll drop ([`77b7d60c4`](https://github.com/lazyxeon/AstraWeave/commit/77b7d60c4)).
- Unified Camera C.7.E: Cinematics chapter documentation closeout ([`ff459e185`](https://github.com/lazyxeon/AstraWeave/commit/ff459e185)).
- Unified Camera C.8: parity harness expansion (first dynamic-workflow sub-phase) ([`2dd65f183`](https://github.com/lazyxeon/AstraWeave/commit/2dd65f183)).
- Unified Camera C.9: campaign closeout (outcome doc + methodology + cleanup hand-off) ([`28858ff29`](https://github.com/lazyxeon/AstraWeave/commit/28858ff29)).

### 2026-05

#### Changed
- Camera System Architecture Audit Sub-phase C.0: audit doc ([`5e03a2ab4`](https://github.com/lazyxeon/AstraWeave/commit/5e03a2ab4)).
- chore: update code structure for improved readability and maintainability ([`a2414c031`](https://github.com/lazyxeon/AstraWeave/commit/a2414c031)).
- CLAUDE.md hardening v0.10.1: silent-Result rule + no-second-pipeline rule + Integration Completeness checklist + Key Lessons tightening ([`28d7a033b`](https://github.com/lazyxeon/AstraWeave/commit/28d7a033b)).
- 5 `docs` commits on `main`, newest "docs: Update engine health status and architecture trace campaign details in README" ([`bdbbbdd61`](https://github.com/lazyxeon/AstraWeave/commit/bdbbbdd61)) through oldest "docs: add architecture trace for Terrain Material System" ([`f3acf3488`](https://github.com/lazyxeon/AstraWeave/commit/f3acf3488)).
- docs(claude.md): add architecture trace contract, wrapped-component axiom, doc hazards ([`a707c3765`](https://github.com/lazyxeon/AstraWeave/commit/a707c3765)).
- Editor-Engine Render Parity P.1: parity harness skeleton (failure-first) ([`1ce3bcf37`](https://github.com/lazyxeon/AstraWeave/commit/1ce3bcf37)).
- Editor-Engine Render Parity P.2: shared loader ([`fc8aad324`](https://github.com/lazyxeon/AstraWeave/commit/fc8aad324)).
- Editor-Engine Render Parity P.3: shared tonemap ([`ecd6b2bfe`](https://github.com/lazyxeon/AstraWeave/commit/ecd6b2bfe)).
- Editor-Engine Render Parity P.4: shared quality preset ([`0c1767d92`](https://github.com/lazyxeon/AstraWeave/commit/0c1767d92)).
- Editor-Engine Render Parity P.5: target format verification ([`f6bf08e7f`](https://github.com/lazyxeon/AstraWeave/commit/f6bf08e7f)).
- Editor-Engine Render Parity P.6: composition layer architecture ([`feeeb4bdd`](https://github.com/lazyxeon/AstraWeave/commit/feeeb4bdd)).
- Editor-Engine Render Parity P.7: parity validation closure ([`57f386c7c`](https://github.com/lazyxeon/AstraWeave/commit/57f386c7c)).
- Editor-Multi-Tool-Architecture.Design.A: campaign-design pass ground-truth doc ([`5bc9b2722`](https://github.com/lazyxeon/AstraWeave/commit/5bc9b2722)).
- Editor-Multi-Tool-Architecture.Design.B: cross-reference in Regional Archetype Variation campaign doc ([`6555117d0`](https://github.com/lazyxeon/AstraWeave/commit/6555117d0)).
- Editor-Multi-Tool-Architecture.Design.C: hash-fixup Design commit hashes ([`cf136eb17`](https://github.com/lazyxeon/AstraWeave/commit/cf136eb17)).
- Editor-Multi-Tool-Architecture.Diagnostic.A: Sub-phase 1 pre-implementation feasibility audit ([`4f43ba345`](https://github.com/lazyxeon/AstraWeave/commit/4f43ba345)).
- Editor-Multi-Tool-Architecture.Diagnostic.B: campaign doc Status header + §11 + §12 update ([`d3bb14ab8`](https://github.com/lazyxeon/AstraWeave/commit/d3bb14ab8)).
- Editor-Multi-Tool-Architecture.Diagnostic.C: hash-fixup Diagnostic.B commit hash to b51cc794d ([`c610cdff4`](https://github.com/lazyxeon/AstraWeave/commit/c610cdff4)).
- Editor-Multi-Tool-Architecture.Real-Fix.D-follow-up: remove biome-unreferenced `default` material from MaterialLibrary canonical (22 named + 10 reserved -> 21 named + 11 reserved); 32-slot MaterialLibrary capacity preserved ([`a6757653c`](https://github.com/lazyxeon/AstraWeave/commit/a6757653c)).
- Editor-Multi-Tool-Architecture.Research.A: canonical dispatcher pattern catalog ([`38b64dcf3`](https://github.com/lazyxeon/AstraWeave/commit/38b64dcf3)).
- 2 `Editor-Multi-Tool-Architecture.Research.B` commits on `main`, newest "Editor-Multi-Tool-Architecture.Research.B: hash-fixup Research.B commit hash to 20a7a1dc9" ([`772a5d148`](https://github.com/lazyxeon/AstraWeave/commit/772a5d148)) through oldest "Editor-Multi-Tool-Architecture.Research.B: cross-reference in Regional Archetype Variation campaign doc" ([`7a1838c7a`](https://github.com/lazyxeon/AstraWeave/commit/7a1838c7a)).
- Editor-Multi-Tool-Architecture.Sub-phase-2.Core.A: ActiveTool trait + EventDisposition + supporting types ([`9c557c4cd`](https://github.com/lazyxeon/AstraWeave/commit/9c557c4cd)).
- Editor-Multi-Tool-Architecture.Sub-phase-2.Core.B: Dispatcher struct + register_tool/set_active_tool/dispatch_*_event/update_active_tool methods ([`0a619a650`](https://github.com/lazyxeon/AstraWeave/commit/0a619a650)).
- Editor-Multi-Tool-Architecture.Sub-phase-2.Core.C: module-level unit tests + MockActiveTool fixture (15 scenarios) ([`d98cd47b2`](https://github.com/lazyxeon/AstraWeave/commit/d98cd47b2)).
- Editor-Multi-Tool-Architecture.Sub-phase-2.Core.D: campaign doc Status header + §11 + §12 update ([`49a45f421`](https://github.com/lazyxeon/AstraWeave/commit/49a45f421)).
- Editor-Multi-Tool-Architecture.Sub-phase-2.Core.E: hash-fixup Core.D commit hash to 7752b13f1 ([`a6bd745b9`](https://github.com/lazyxeon/AstraWeave/commit/a6bd745b9)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.A: impl ActiveTool for TerrainPanel + TerrainAction::SetActiveTool variant ([`35f677448`](https://github.com/lazyxeon/AstraWeave/commit/35f677448)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.B: tab_viewer SetActiveTool capture + EditorApp dispatcher field + ViewportWidget cached-then-dispatch integration ([`93d1515c2`](https://github.com/lazyxeon/AstraWeave/commit/93d1515c2)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.C.Closeout: campaign chain consolidation; Sub-phase 3 architectural objective + cleanup chain COMPLETE; methodology body of practice carry-forward established ([`65bfc1860`](https://github.com/lazyxeon/AstraWeave/commit/65bfc1860)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Cleanup-A: delete dead-code legacy cluster path; canonical pipeline (Real-Fix.A/B/C/D/E) is single source of truth for terrain rendering ([`a63871a1f`](https://github.com/lazyxeon/AstraWeave/commit/a63871a1f)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Cleanup-B: revert Mediator Brush Fix 93b0f5843 (NO-OP MARKER); pre-execution found entire 93b0f5843 content is canonical pipeline foundation, not bypass-mechanism; nothing to revert ([`f474af93b`](https://github.com/lazyxeon/AstraWeave/commit/f474af93b)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Cleanup-D.Fix-Pass: state-propagation pathway equivalence at three sites; M-D5 cross-chunk avg_height + M-D9 post-brush stitch_edge_normals + M-SK skirt vertex height update; eliminates erosion chunk seam + lighting seam + skirt decoupling ([`9859b1f06`](https://github.com/lazyxeon/AstraWeave/commit/9859b1f06)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Cleanup-D.Research-Pass: analytical-only chunk-boundary continuity layer characterization per (n) n-3; M-D5 erosion avg_height divergence + M-D9 post-brush normal-stitching gap + M-SK skirt height freeze identified; q-1 direct fix recommended ([`1b38e6232`](https://github.com/lazyxeon/AstraWeave/commit/1b38e6232)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Cleanup-D.Research-Pass.B: hash-fixup Cleanup-D.Research-Pass.A commit hash to 3710b1f1c ([`cfa2744e1`](https://github.com/lazyxeon/AstraWeave/commit/cfa2744e1)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Diagnostic.A: TerrainPanel brush mediator path failure-diagnosis audit ([`17a6b8e61`](https://github.com/lazyxeon/AstraWeave/commit/17a6b8e61)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Diagnostic.B: campaign doc Status header + §11 + §12 update ([`14febada3`](https://github.com/lazyxeon/AstraWeave/commit/14febada3)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Diagnostic.C: hash-fixup Diagnostic.B commit hash to 2a03f0bc2 ([`0b1e04547`](https://github.com/lazyxeon/AstraWeave/commit/0b1e04547)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Instrumentation.A: add 5 eprintln points for runtime evidence capture ([`294eee8e2`](https://github.com/lazyxeon/AstraWeave/commit/294eee8e2)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Instrumentation.B: revert Round 1 eprintln points; production code returns to post-fix state ([`1f44a9767`](https://github.com/lazyxeon/AstraWeave/commit/1f44a9767)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-1-Closure.A: §12 entry capturing Round 1 instrumentation findings + outcome classification + methodology lesson ([`8d8ce251d`](https://github.com/lazyxeon/AstraWeave/commit/8d8ce251d)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-1-Closure.B: hash-fixup Round-1-Closure.A commit hash to 365c97602 ([`d19de7bc8`](https://github.com/lazyxeon/AstraWeave/commit/d19de7bc8)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-2-Instrumentation.A: add 6 targeted eprintlns (filtered) for Defect A + Defect B disambiguation ([`63ced951b`](https://github.com/lazyxeon/AstraWeave/commit/63ced951b)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-2-Instrumentation.B: revert Round 2 eprintln points; production code returns to post-fix state ([`941c0256a`](https://github.com/lazyxeon/AstraWeave/commit/941c0256a)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-4-Closure.A: codify Round 4 captured evidence with corrected analysis; supersede 928979301 misleading commit body ([`b35fb0846`](https://github.com/lazyxeon/AstraWeave/commit/b35fb0846)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-4-Closure.B: hash-fixup Round-4-Closure.A commit hash to 5517944bb ([`2ecda1f54`](https://github.com/lazyxeon/AstraWeave/commit/2ecda1f54)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-4-Instrumentation.A: add 12 targeted eprintln points across 8 hypothesis classes per Round 3 audit §4.2 ([`cbcf02be4`](https://github.com/lazyxeon/AstraWeave/commit/cbcf02be4)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-4-Instrumentation.B: revert Round 4 eprintln points; production code returns to post-Round-3 state ([`200ef296a`](https://github.com/lazyxeon/AstraWeave/commit/200ef296a)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-5-Closure.A: codify Round 5 captured evidence; Mechanism 1 ("wrong texture, different render target" sub-class) DEFINITIVELY CONFIRMED; present three fix candidates for Andrew-gate decision ([`6b17ed412`](https://github.com/lazyxeon/AstraWeave/commit/6b17ed412)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-5-Closure.B: hash-fixup Round-5-Closure.A commit hash to c214d26a5 ([`55ebfa396`](https://github.com/lazyxeon/AstraWeave/commit/55ebfa396)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-5-Instrumentation.A: add 5 eprintlns across depth pick chain to runtime-verify mechanism within Round-4-Closure-identified defect class ([`ea3d3c58e`](https://github.com/lazyxeon/AstraWeave/commit/ea3d3c58e)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-5-Instrumentation.B: revert 5 eprintlns across depth pick chain; production code returns to post-928979301 state ([`4d97258ed`](https://github.com/lazyxeon/AstraWeave/commit/4d97258ed)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-6-Closure.A: codify Round 6 captured evidence; Mechanism C (mesh resource identity trap, sibling §7.7 at mesh-data layer) DEFINITIVELY CONFIRMED with dual-variant sub-mechanism; §7.7 graduates from CANDIDATE to CONFIRMED CANONICAL PATTERN with multi-layer evidence; present two fix candidates for Andrew-gate decision ([`619f27ca0`](https://github.com/lazyxeon/AstraWeave/commit/619f27ca0)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-6-Closure.B: hash-fixup Round-6-Closure.A commit hash to e1254b6ce ([`33e2286bb`](https://github.com/lazyxeon/AstraWeave/commit/33e2286bb)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-6-Instrumentation.A: add 6 eprintlns across mesh-update chain to runtime-verify mechanism within Andrew-gate partial-REGRESS hypothesis space ([`f3081dbba`](https://github.com/lazyxeon/AstraWeave/commit/f3081dbba)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-6-Instrumentation.B: revert 6 eprintlns across mesh-update chain; production code returns to post-176e5ab0c state ([`765559a71`](https://github.com/lazyxeon/AstraWeave/commit/765559a71)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-7-Closure.A: codify Round 7 captured evidence; Mechanism H1 (wrapped-component sibling-attribute drift; intra-component variant of §7.7 at texture-data layer) DEFINITIVELY CONFIRMED with smoking-gun chunk_index=40 paint sequence; §7.7 ELEVATES from canonical pattern to fundamental architectural anti-pattern with three-layer evidence + intra-component sub-variant + two granularity scales; present three fix candidates for paint + ZoneBlend bundled-vs-split scoping for Andrew-gate decisions ([`a46fa8f24`](https://github.com/lazyxeon/AstraWeave/commit/a46fa8f24)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-7-Closure.B: hash-fixup Round-7-Closure.A commit hash to 3914b0bc7 ([`16a7c564c`](https://github.com/lazyxeon/AstraWeave/commit/16a7c564c)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-7-Instrumentation.A: add 5 eprintlns (T10.A-E) across paint/zoneblend texture-mutation chain to runtime-verify partial-REGRESS hypothesis space (paint+zoneblend invisible despite Real-Fix.B e8f30c6bb fixing 6/8 brush modes) ([`138898e0e`](https://github.com/lazyxeon/AstraWeave/commit/138898e0e)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-7-Instrumentation.B: revert 5 eprintlns across paint/zoneblend texture-mutation chain; production code returns to post-Real-Fix.B e8f30c6bb state ([`590da3ec7`](https://github.com/lazyxeon/AstraWeave/commit/590da3ec7)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-8-Closure.A: codify Round 8 captured evidence; Mechanism H8.1 (UI/renderer material library capacity boundary mismatch; FOURTH §7.7 instance in same campaign) DEFINITIVELY CONFIRMED with smoking-gun first-non-working-onset at rock_slate idx=8 + 100% correlation across 13 tested non-working materials and 8 working materials; §7.7 ELEVATES from fundamental architectural anti-pattern to STRUCTURAL AXIOM with four-layer evidence + three granularity scales; present three fix candidates for Andrew-gate decision (h) ([`15065043b`](https://github.com/lazyxeon/AstraWeave/commit/15065043b)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-8-Closure.B: hash-fixup Round-8-Closure.A commit hash to 65263ee41 ([`bac2d1e07`](https://github.com/lazyxeon/AstraWeave/commit/bac2d1e07)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-8-Instrumentation.A: add 4 eprintlns across material library coverage chain to runtime-verify mechanism within Defect Class 4 (Material Library Coverage Gap) ([`d7c71b53c`](https://github.com/lazyxeon/AstraWeave/commit/d7c71b53c)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Diagnostic-Round-8-Instrumentation.B: revert 4 eprintlns across material library coverage chain; production code returns to post-Real-Fix.C 52efe80b8 state ([`e782744c2`](https://github.com/lazyxeon/AstraWeave/commit/e782744c2)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Fix.A: relocate set_terrain_brush_active + set_terrain_brush_params outside Some(world) gate ([`e3f5a2bfe`](https://github.com/lazyxeon/AstraWeave/commit/e3f5a2bfe)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Real-Fix.A: pass depth attachment to engine_adapter::render_to_texture (Option (a) per Andrew-gate decision); resolves Mechanism 1 (wrong texture) confirmed by Round 5 evidence ([`4fed10116`](https://github.com/lazyxeon/AstraWeave/commit/4fed10116)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Real-Fix.B: extract upload_or_update_terrain_chunk_forward shared helper (Option 1 per Andrew-gate decision); resolves Mechanism C (mesh resource identity trap) confirmed by Round 6 evidence ([`1cdacdeeb`](https://github.com/lazyxeon/AstraWeave/commit/1cdacdeeb)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Real-Fix.C: unify TerrainVertex biome_weights_* and material_* into single canonical material_* attribute set (Option C per Andrew-gate decision); resolves Mechanism H1 (sibling-attribute drift; intra-component §7.7 instance at texture-data layer) confirmed by Round 7 evidence ([`7f52ee226`](https://github.com/lazyxeon/AstraWeave/commit/7f52ee226)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Real-Fix.D: canonical material library shared between UI + renderer; resolves Defect Class 4 (UI/renderer material library capacity boundary mismatch; FOURTH §7.7 instance) per Andrew-gate decision (h) Option D-2 ([`d038147a3`](https://github.com/lazyxeon/AstraWeave/commit/d038147a3)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Real-Fix.E: ZoneBlend brush mode implementation; active brush + passive global pass; biome boundary feathering per Andrew-gate (l) l-1 + (m) m-3 ([`c4a33bd4c`](https://github.com/lazyxeon/AstraWeave/commit/c4a33bd4c)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Round-3-Re-Diagnostic.A: formal re-diagnostic audit synthesizing three rounds of prior runtime evidence ([`e9c8ee2af`](https://github.com/lazyxeon/AstraWeave/commit/e9c8ee2af)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Round-3-Re-Diagnostic.B: campaign doc Status header + §11 + §12 update (Round-2-Closure + Round-3-Re-Diagnostic) ([`5f2563637`](https://github.com/lazyxeon/AstraWeave/commit/5f2563637)).
- Editor-Multi-Tool-Architecture.Sub-phase-3.Mediator-Brush-Round-3-Re-Diagnostic.C: hash-fixup Round-3-Re-Diagnostic.B commit hash to 5ea4f2074 ([`074b0d747`](https://github.com/lazyxeon/AstraWeave/commit/074b0d747)).
- 10 `misc` commits on `main`, newest "Refactor bind group cache and render graph to eliminate unnecessary unwraps; improve safety with let-else pattern in instancing and terrain material manager; optimize pipeline creation logic; enhance vertex smoothing in chunk management; introduce constant spline helper method in spline types." ([`ea889f2ee`](https://github.com/lazyxeon/AstraWeave/commit/ea889f2ee)) through oldest "Add atlas configuration files for new species" ([`5e06d26d7`](https://github.com/lazyxeon/AstraWeave/commit/5e06d26d7)).
- 2 `Phase 1.X-F.3.D` commits on `main`, newest "Phase 1.X-F.3.D: hash-fixup F.3.D commit hash to f47d87d61" ([`5368a3c66`](https://github.com/lazyxeon/AstraWeave/commit/5368a3c66)) through oldest "Phase 1.X-F.3.D: close F.3 (Andrew-gate PASS)" ([`32db759ce`](https://github.com/lazyxeon/AstraWeave/commit/32db759ce)).
- Phase 1.X-F.4.A: RegionalArchetypeMask type + RON metadata ([`04c007464`](https://github.com/lazyxeon/AstraWeave/commit/04c007464)).
- Phase 1.X-F.4.B: mask save/load + Euclidean distance transform ([`217ea10fe`](https://github.com/lazyxeon/AstraWeave/commit/217ea10fe)).
- Phase 1.X-F.4.C: RegionalArchetypeBlend neighborhood-scan sampler ([`de7fabbc5`](https://github.com/lazyxeon/AstraWeave/commit/de7fabbc5)).
- Phase 1.X-F.4.D: multi-archetype blend math (BootstrapParams) ([`a1f7f9712`](https://github.com/lazyxeon/AstraWeave/commit/a1f7f9712)).
- Phase 1.X-F.4.E: wire RegionalArchetypeMask into WorldGenerator ([`8550bf8d5`](https://github.com/lazyxeon/AstraWeave/commit/8550bf8d5)).
- Phase 1.X-F.4.F: F.4 integration + perf + synthetic-differentiation tests ([`e36a0e3da`](https://github.com/lazyxeon/AstraWeave/commit/e36a0e3da)).
- Phase 1.X-F.5-paint.A: RegionalArchetypePanel scaffold ([`657cb2907`](https://github.com/lazyxeon/AstraWeave/commit/657cb2907)).
- Phase 1.X-F.5-paint.B: brush implementation + falloff visualization ([`84ec2a45b`](https://github.com/lazyxeon/AstraWeave/commit/84ec2a45b)).
- Phase 1.X-F.5-paint.C: mask save/load + sibling-directory project layout ([`01990ece3`](https://github.com/lazyxeon/AstraWeave/commit/01990ece3)).
- 2 `Phase 1.X-F.5-paint.D` commits on `main`, newest "Phase 1.X-F.5-paint.D: hash-fixup F.5-paint.D commit hash to 0c99aee88" ([`4c2efb3ec`](https://github.com/lazyxeon/AstraWeave/commit/4c2efb3ec)) through oldest "Phase 1.X-F.5-paint.D: close F.5-paint slice" ([`016b76c2a`](https://github.com/lazyxeon/AstraWeave/commit/016b76c2a)).
- Phase 1.X-F.5-paint.E-diagnostic.A: panel registration audit ([`277d75912`](https://github.com/lazyxeon/AstraWeave/commit/277d75912)).
- 2 `Phase 1.X-F.5-paint.E-diagnostic.B` commits on `main`, newest "Phase 1.X-F.5-paint.E-diagnostic.B: hash-fixup F.5-paint.E-diagnostic.B commit hash to 375e397b7" ([`45e68ca4a`](https://github.com/lazyxeon/AstraWeave/commit/45e68ca4a)) through oldest "Phase 1.X-F.5-paint.E-diagnostic.B: campaign doc revert + REGRESS log" ([`c58d8bb09`](https://github.com/lazyxeon/AstraWeave/commit/c58d8bb09)).
- Phase 1.X-F.5-paint.F-fix.A: register RegionalArchetype panel + Pattern A regression tests ([`74e15bc7b`](https://github.com/lazyxeon/AstraWeave/commit/74e15bc7b)).
- Phase 1.X-F.5-paint.F-fix.A-supplement: cover exhaustive-match dispatch sites missed by audit ([`7e1ab5cc6`](https://github.com/lazyxeon/AstraWeave/commit/7e1ab5cc6)).
- 2 `Phase 1.X-F.5-paint.F-fix.B` commits on `main`, newest "Phase 1.X-F.5-paint.F-fix.B: hash-fixup F.5-paint.F-fix.B commit hash to 680724f4d" ([`d1d56dd5f`](https://github.com/lazyxeon/AstraWeave/commit/d1d56dd5f)) through oldest "Phase 1.X-F.5-paint.F-fix.B: close F-fix (Andrew-gate PASS) + audit amendment" ([`020f95ca5`](https://github.com/lazyxeon/AstraWeave/commit/020f95ca5)).
- Phase 1.X-G-pointer-events-diagnostic.A: code investigation + (B-extend)/(A)/(C)/hybrid decision request ([`fce684a19`](https://github.com/lazyxeon/AstraWeave/commit/fce684a19)).
- 2 `Phase 1.X-G-pointer-events-diagnostic.B` commits on `main`, newest "Phase 1.X-G-pointer-events-diagnostic.B: hash-fixup G-diagnostic.B commit hash to 01cee9a6a" ([`2b2a2f046`](https://github.com/lazyxeon/AstraWeave/commit/2b2a2f046)) through oldest "Phase 1.X-G-pointer-events-diagnostic.B: campaign doc update for G-diagnostic session" ([`afff61c49`](https://github.com/lazyxeon/AstraWeave/commit/afff61c49)).
- Phase 1.X-G-research.A: SOTA research on pointer-event dispatch + multi-tool arbitration ([`09dc90fdc`](https://github.com/lazyxeon/AstraWeave/commit/09dc90fdc)).
- 2 `Phase 1.X-G-research.B` commits on `main`, newest "Phase 1.X-G-research.B: hash-fixup G-research.B commit hash to e7b699947" ([`2aa7e009d`](https://github.com/lazyxeon/AstraWeave/commit/2aa7e009d)) through oldest "Phase 1.X-G-research.B: campaign doc update for G-research session" ([`b1df6b00f`](https://github.com/lazyxeon/AstraWeave/commit/b1df6b00f)).
- Phase 1.X-pause.A: F.4 closeout (Andrew-gate PASS deferred-closeout reconciliation) ([`c531b8ea4`](https://github.com/lazyxeon/AstraWeave/commit/c531b8ea4)).
- Phase 1.X-pause.B: F.5-paint pause + Editor Multi-Tool Architecture spinoff ([`a2113054c`](https://github.com/lazyxeon/AstraWeave/commit/a2113054c)).
- Phase 1.X-pause.C: hash-fixup Pause-Artifacts.B commit hash to 3d7a8d07c ([`a60327fa6`](https://github.com/lazyxeon/AstraWeave/commit/a60327fa6)).
- Terrain Asset Quality A.1.B: 2 verified-slug materials acquired ([`ea6c08579`](https://github.com/lazyxeon/AstraWeave/commit/ea6c08579)).
- Terrain Asset Quality A.1.C: ice acquired from ambientCG (scaffolding) ([`be06e9939`](https://github.com/lazyxeon/AstraWeave/commit/be06e9939)).
- Terrain Asset Quality A.3: grassland validate-only (premise falsified) ([`39c84f94b`](https://github.com/lazyxeon/AstraWeave/commit/39c84f94b)).
- Terrain-Asset-Quality.Research-Pass: analytical-only campaign launch; characterize current asset state + pipeline + organization options + Andrew-gate decisions surfaced; methodology body of practice inherited from Editor Multi-Tool Architecture Sub-phase 3 ([`fcc7aa5c0`](https://github.com/lazyxeon/AstraWeave/commit/fcc7aa5c0)).
- Terrain-Asset-Quality.Sub-phase-A.0.A: extend astraweave-assets PolyHaven provider with "arm" map type; restores audit sect 3.1 rename-only conversion premise; Sub-phase A.1 acquisition UNBLOCKED ([`e1daaeceb`](https://github.com/lazyxeon/AstraWeave/commit/e1daaeceb)).
- Terrain-Asset-Quality.Sub-phase-A.1: 6-material source acquisition (5 fetched + 1 reused); rename-only + jpg-to-png conversion per audit sect 3.1; canonical baseline preserved at 21 named; 3 deferred to A.1.B (HTTP 404 on 7f5381139-recommended IDs) ([`d68f137b7`](https://github.com/lazyxeon/AstraWeave/commit/d68f137b7)).
- Terrain-Asset-Quality.Sub-phase-A.1.Marker: BLOCKED pending fetcher capability extension; pre-execution surfaced PolyHaven provider gap (no ARM-packed map fetch) + missing ImageMagick tooling; Andrew-gate chose Pause+extend fetcher path; default material reclassified per Andrew-gate (iii) as dead-code for separate cleanup ([`e5ca3e5dc`](https://github.com/lazyxeon/AstraWeave/commit/e5ca3e5dc)).
- Unified Camera C.1: canonical convention lockdown ([`a471cc7a3`](https://github.com/lazyxeon/AstraWeave/commit/a471cc7a3)).
- Unified Camera C.2: astraweave-camera crate exists with canonical types ([`8c551c6ea`](https://github.com/lazyxeon/AstraWeave/commit/8c551c6ea)).
- Unified Camera C.3.A: FreeFly migration + new renderer API ([`39db36f3d`](https://github.com/lazyxeon/AstraWeave/commit/39db36f3d)).
- Unified Camera C.3.B.1: editor-side caller migration ([`e26e32f7b`](https://github.com/lazyxeon/AstraWeave/commit/e26e32f7b)).
- Unified Camera C.3.B.2: engine-side caller migration ([`b790950a0`](https://github.com/lazyxeon/AstraWeave/commit/b790950a0)).
- Unified Camera C.3.C: deprecated path deletion + canonical rename ([`a05c824ac`](https://github.com/lazyxeon/AstraWeave/commit/a05c824ac)).
- Unified Camera C.3.D: renderer-side chapter documentation closeout ([`9695836c5`](https://github.com/lazyxeon/AstraWeave/commit/9695836c5)).
- Unified Camera C.3.E: update sanitizer workflow to include nightly runs for critical crates, refactor heightmap imports, and enhance noise configuration tests ([`0d462d99b`](https://github.com/lazyxeon/AstraWeave/commit/0d462d99b)).
- Unified Camera C.4: OrbitCamera adopts CameraProducer + picking fix + docs ([`0326d5803`](https://github.com/lazyxeon/AstraWeave/commit/0326d5803)).
- Unified Camera C.4.B: OrbitCamera fov → fovy field rename ([`810447bae`](https://github.com/lazyxeon/AstraWeave/commit/810447bae)).
- Unified Camera C.5: Phase 2 audit (gizmo + per-example + cinematics inventory) ([`65fb6e767`](https://github.com/lazyxeon/AstraWeave/commit/65fb6e767)).
- Unified Camera C.6: Per-target migration queue execution (combined) ([`b7f80566a`](https://github.com/lazyxeon/AstraWeave/commit/b7f80566a)).
- Unified Camera C.7.0: Cinematics consolidation preliminary audit ([`e758e3021`](https://github.com/lazyxeon/AstraWeave/commit/e758e3021)).
- Unified Camera C.7.A: Cue::CameraTo look_at migration + CutsceneTickEvent ([`2eafe3c56`](https://github.com/lazyxeon/AstraWeave/commit/2eafe3c56)).
- Unified Camera C.7.B: cutscene_render_demo tick loop rewrite ([`033996c07`](https://github.com/lazyxeon/AstraWeave/commit/033996c07)).
- Unified Camera C.7.D: apply_camera_key boundary hardening + CameraKey::sanitize ([`bb96b24a8`](https://github.com/lazyxeon/AstraWeave/commit/bb96b24a8)).

### 2026-04

#### Changed
- Add new asset: verdant_trail.blend ([`3ad2b6294`](https://github.com/lazyxeon/AstraWeave/commit/3ad2b6294)).
- 3 `docs` commits on `main`, newest "docs: add commit reference for ParallelSchedule removal audit" ([`17be8222d`](https://github.com/lazyxeon/AstraWeave/commit/17be8222d)) through oldest "docs: update README, CLAUDE.md, Architecture Map, and Master Roadmap" ([`838a8a386`](https://github.com/lazyxeon/AstraWeave/commit/838a8a386)).
- 14 `docs(editor)` commits on `main`, newest "docs(editor): CAMPAIGN COMPLETE — 35 fixes, 44 commits, unified pipeline" ([`49a0b2ea0`](https://github.com/lazyxeon/AstraWeave/commit/49a0b2ea0)) through oldest "docs(editor): update campaign progress — Tier 2 fixes 6-11 complete" ([`f5ba18813`](https://github.com/lazyxeon/AstraWeave/commit/f5ba18813)).
- docs(editor/adapter): document terrain vertex data simplification — Fix-18 ([`c3eab13d1`](https://github.com/lazyxeon/AstraWeave/commit/c3eab13d1)).
- docs(readme): update engine health status to April 2026 — grade B+ to A- ([`af544f24c`](https://github.com/lazyxeon/AstraWeave/commit/af544f24c)).
- 4 `docs(terrain-mat-campaign)` commits on `main`, newest "docs(terrain-mat-campaign): backfill real 1.F commit hash in §7" ([`7aebda8d9`](https://github.com/lazyxeon/AstraWeave/commit/7aebda8d9)) through oldest "docs(terrain-mat-campaign): log Phase 1 deviation — splat pipeline is deferred G-buffer" ([`e1fadbe10`](https://github.com/lazyxeon/AstraWeave/commit/e1fadbe10)).
- 4 `feat` commits on `main`, newest "feat: integrate mimalloc as an optional global allocator across AstraWeave" ([`ebdc740eb`](https://github.com/lazyxeon/AstraWeave/commit/ebdc740eb)) through oldest "feat: Implement post-processing chain for editor viewport" ([`55735d274`](https://github.com/lazyxeon/AstraWeave/commit/55735d274)).
- feat(console): implement tracing integration for the editor console ([`07b3d2137`](https://github.com/lazyxeon/AstraWeave/commit/07b3d2137)).
- feat(core/editor): add per-axis scale support to Pose — Fix-17 ([`ae7e10ff3`](https://github.com/lazyxeon/AstraWeave/commit/ae7e10ff3)).
- 4 `feat(editor)` commits on `main`, newest "feat(editor): delete legacy EntityRenderer + 5 editor shaders — Fix-27 Phase 6b" ([`25eeca0ff`](https://github.com/lazyxeon/AstraWeave/commit/25eeca0ff)) through oldest "feat(editor): make astraweave-render non-optional + remove all cfg guards — Fix-27 Phase 0" ([`bacb8d510`](https://github.com/lazyxeon/AstraWeave/commit/bacb8d510)).
- feat(editor/engine): feed World entities to engine renderer — Fix-27 Phase 1a ([`159cac7c8`](https://github.com/lazyxeon/AstraWeave/commit/159cac7c8)).
- feat(editor/shader): add Khronos PBR Neutral + Reinhard tonemappers — Fix-22 ([`9f4bf3e80`](https://github.com/lazyxeon/AstraWeave/commit/9f4bf3e80)).
- feat(editor/terrain): add biome-aware terrain vertex conversion — Fix-27 Phase 5 ([`8ec0505ac`](https://github.com/lazyxeon/AstraWeave/commit/8ec0505ac)).
- feat(editor/viewport): add headless fallback when engine adapter unavailable — Fix-27 Phase 7 ([`b2c4f705f`](https://github.com/lazyxeon/AstraWeave/commit/b2c4f705f)).
- feat(renderer): add HDR passthrough blit pipeline for editor mode ([`3c7544144`](https://github.com/lazyxeon/AstraWeave/commit/3c7544144)).
- fix(camera): adjust camera direction handling to prevent rendering issues fix(renderer): update shadow mapping to use camera position for cascade selection fix(heightmap): correct triangle index generation for counter-clockwise winding test(heightmap): update tests to reflect changes in triangle index generation fix(editor): modify camera update to use direct view/proj matrices from OrbitCamera ([`f4c047665`](https://github.com/lazyxeon/AstraWeave/commit/f4c047665)).
- 2 `fix(ci)` commits on `main`, newest "fix(ci): resolve 3 integration test failures" ([`dd5d21042`](https://github.com/lazyxeon/AstraWeave/commit/dd5d21042)) through oldest "fix(ci): add installation of system dependencies for sanitizer workflow" ([`47e1ef483`](https://github.com/lazyxeon/AstraWeave/commit/47e1ef483)).
- fix(ecs): register 8 canonical stages in App::new and warn on unknown-stage add_system ([`507b698ec`](https://github.com/lazyxeon/AstraWeave/commit/507b698ec)).
- 3 `fix(editor)` commits on `main`, newest "fix(editor): resolve remaining audit loose ends — case mismatch, warnings, cleanup" ([`67f4baf2a`](https://github.com/lazyxeon/AstraWeave/commit/67f4baf2a)) through oldest "fix(editor): surface errors for material preview and asset scan — C-1, C-2" ([`40867b38d`](https://github.com/lazyxeon/AstraWeave/commit/40867b38d)).
- fix(editor/gizmo): enable bidirectional scaling via signed mouse delta — M-21 ([`dd8639513`](https://github.com/lazyxeon/AstraWeave/commit/dd8639513)).
- fix(editor/runtime): catch simulation panics to prevent editor crash — Fix-11 ([`8adfd26b7`](https://github.com/lazyxeon/AstraWeave/commit/8adfd26b7)).
- fix(editor/tonemap): separate tonemap paths for engine vs FastPreview — Fix-27 Phase 3 ([`b8dec997f`](https://github.com/lazyxeon/AstraWeave/commit/b8dec997f)).
- 3 `fix(editor/undo)` commits on `main`, newest "fix(editor/undo): wire all 5 component operations through undo stack — C-5 complete" ([`6b3fd279f`](https://github.com/lazyxeon/AstraWeave/commit/6b3fd279f)) through oldest "fix(editor/undo): remove duplicate EntityManager updates that bypass undo — Fix-12" ([`18ea8298b`](https://github.com/lazyxeon/AstraWeave/commit/18ea8298b)).
- fix(editor/viewport): eliminate mutex poison cascade with recovering helper — C-3 ([`95260daed`](https://github.com/lazyxeon/AstraWeave/commit/95260daed)).
- fix(render): improve error handling in bind group creation and output view retrieval ([`6eb05efe6`](https://github.com/lazyxeon/AstraWeave/commit/6eb05efe6)).
- 22 `misc` commits on `main`, newest "Update scheduled_tasks.lock and add CAMERA_SYSTEMS_SOTA_AUDIT_AND_RECOMMENDATIONS.md for camera system documentation" ([`5b370a37b`](https://github.com/lazyxeon/AstraWeave/commit/5b370a37b)) through oldest "Implement Temporal Anti-Aliasing (TAA) and Velocity Buffer for motion vector generation" ([`4c8817b45`](https://github.com/lazyxeon/AstraWeave/commit/4c8817b45)).
- 4 `Phase 1 post-completion` commits on `main`, newest "Phase 1 post-completion: close — re-mark COMPLETE after triangle-streak fix" ([`5e1a8a3f3`](https://github.com/lazyxeon/AstraWeave/commit/5e1a8a3f3)) through oldest "Phase 1 post-completion: revert COMPLETE status pending triangle-streak remediation" ([`3ab9f211a`](https://github.com/lazyxeon/AstraWeave/commit/3ab9f211a)).
- 3 `Phase 1 re-cleanup` commits on `main`, newest "Phase 1 re-cleanup: log Issue 2 investigation findings in §9 (deferred)" ([`62af670a6`](https://github.com/lazyxeon/AstraWeave/commit/62af670a6)) through oldest "Phase 1 re-cleanup: revert Phase 1 and Phase 1.5 COMPLETE status pending remediation" ([`67e567390`](https://github.com/lazyxeon/AstraWeave/commit/67e567390)).
- Phase 1.0: amend campaign plan for Option D (forward-lit splat shader) ([`75f5af193`](https://github.com/lazyxeon/AstraWeave/commit/75f5af193)).
- Phase 1.5-T.A: heightmap range investigation ([`2e94d25cc`](https://github.com/lazyxeon/AstraWeave/commit/2e94d25cc)).
- Phase 1.5-T.C: retune elevation band constants for observed heightmap Y range ([`4dfb8b9f8`](https://github.com/lazyxeon/AstraWeave/commit/4dfb8b9f8)).
- Phase 1.5-T.E: closeout — tuned multi-biome terrain active ([`1fe29c7c4`](https://github.com/lazyxeon/AstraWeave/commit/1fe29c7c4)).
- Phase 1.5.A: amend campaign plan to add Phase 1.5 spec ([`c104f42a8`](https://github.com/lazyxeon/AstraWeave/commit/c104f42a8)).
- Phase 1.5.B: add elevation_to_biome_weights and ClimateBias to astraweave-terrain ([`386159516`](https://github.com/lazyxeon/AstraWeave/commit/386159516)).
- Phase 1.5.C: wire elevation_to_biome_weights into chunk generation ([`4740b4b1e`](https://github.com/lazyxeon/AstraWeave/commit/4740b4b1e)).
- Phase 1.5.E: complete — multi-biome terrain generation active ([`7592ccd05`](https://github.com/lazyxeon/AstraWeave/commit/7592ccd05)).
- 2 `Phase 1.6-F closure` commits on `main`, newest "Phase 1.6-F closure: hash-fixup to a95c1a4c9" ([`7e2f5cb57`](https://github.com/lazyxeon/AstraWeave/commit/7e2f5cb57)) through oldest "Phase 1.6-F closure: F.4.B.3.D + F.5 + parent campaign + ARCHITECTURE_MAP" ([`5a43b1ad7`](https://github.com/lazyxeon/AstraWeave/commit/5a43b1ad7)).
- Phase 1.6-F.0: draft terrain generation quality campaign plan ([`c8b623c35`](https://github.com/lazyxeon/AstraWeave/commit/c8b623c35)).
- Phase 1.6-F.1.A: tune grassland preset to 100+ unit Y span ([`6f70119b1`](https://github.com/lazyxeon/AstraWeave/commit/6f70119b1)).
- Phase 1.6-F.1.B: tune remaining seven biome-noise presets ([`c74ca6d33`](https://github.com/lazyxeon/AstraWeave/commit/c74ca6d33)).
- 2 `Phase 1.6-F.1.C` commits on `main`, newest "Phase 1.6-F.1.C: fix §9 F.1.C commit hash to dcf7d02ff" ([`858e09c0c`](https://github.com/lazyxeon/AstraWeave/commit/858e09c0c)) through oldest "Phase 1.6-F.1.C: close F.1, Phase 1.5-T correction note" ([`b828068fd`](https://github.com/lazyxeon/AstraWeave/commit/b828068fd)).
- Phase 1.6-F.2: amend plan — continental-scale macro-feature, scope expansion ([`d2f8b8ad2`](https://github.com/lazyxeon/AstraWeave/commit/d2f8b8ad2)).
- Phase 1.6-F.2-T-2.A: per-layer frequency-content diagnostic ([`d216425e0`](https://github.com/lazyxeon/AstraWeave/commit/d216425e0)).
- Phase 1.6-F.2-T-2.B.3: halve DomainWarp warp_strength on five presets + exploratory matrix ([`83ec073cc`](https://github.com/lazyxeon/AstraWeave/commit/83ec073cc)).
- Phase 1.6-F.2-T-2.C: verify diagnostic improvement + add spike regression test ([`3ccbe99a7`](https://github.com/lazyxeon/AstraWeave/commit/3ccbe99a7)).
- 2 `Phase 1.6-F.2-T-2.D` commits on `main`, newest "Phase 1.6-F.2-T-2.D: fix §9 + §10 F.2-T-2.D hash references to b2d521a4c" ([`71d6a5cbe`](https://github.com/lazyxeon/AstraWeave/commit/71d6a5cbe)) through oldest "Phase 1.6-F.2-T-2.D: close F.2 second tuning pass + plan updates" ([`a0532bc63`](https://github.com/lazyxeon/AstraWeave/commit/a0532bc63)).
- Phase 1.6-F.2-T-3.A: terrain noise spike research ([`34bd85ac7`](https://github.com/lazyxeon/AstraWeave/commit/34bd85ac7)).
- Phase 1.6-F.2-T-3.B: terrain noise code audit ([`f100b0da4`](https://github.com/lazyxeon/AstraWeave/commit/f100b0da4)).
- Phase 1.6-F.2-T-3.C.1: cap base_octaves per PBR Nyquist formula ([`4e4d21c97`](https://github.com/lazyxeon/AstraWeave/commit/4e4d21c97)).
- 2 `Phase 1.6-F.2-T-3.D` commits on `main`, newest "Phase 1.6-F.2-T-3.D: fix §9 + §10 F.2-T-3.D hash references to 694dd4a2a" ([`abf585d09`](https://github.com/lazyxeon/AstraWeave/commit/abf585d09)) through oldest "Phase 1.6-F.2-T-3.D: close research + audit pass" ([`c94d17ca4`](https://github.com/lazyxeon/AstraWeave/commit/c94d17ca4)).
- Phase 1.6-F.2-T-4.A+B: analytical-derivative Perlin + derivative-weighted fBm ([`284484336`](https://github.com/lazyxeon/AstraWeave/commit/284484336)).
- Phase 1.6-F.2-T-4.C+D: wire derivative-weighted fBm into TerrainNoise base layer ([`0cc6f2956`](https://github.com/lazyxeon/AstraWeave/commit/0cc6f2956)).
- 2 `Phase 1.6-F.2-T-4.E` commits on `main`, newest "Phase 1.6-F.2-T-4.E: fix §9 + §10 F.2-T-4.E hash references to d6b070d5e" ([`641ea7846`](https://github.com/lazyxeon/AstraWeave/commit/641ea7846)) through oldest "Phase 1.6-F.2-T-4.E: close F.2-T-4 + plan updates" ([`440843093`](https://github.com/lazyxeon/AstraWeave/commit/440843093)).
- Phase 1.6-F.2-T.A: continental and layer-contribution diagnostic ([`256cd06f8`](https://github.com/lazyxeon/AstraWeave/commit/256cd06f8)).
- Phase 1.6-F.2-T.B.1: tune NoiseConfig continental parameters per F.2-T diagnostic ([`0dba92b73`](https://github.com/lazyxeon/AstraWeave/commit/0dba92b73)).
- Phase 1.6-F.2-T.B.2: reduce detail_amplitude on five DomainWarped presets per F.2-T diagnostic ([`38c585b7b`](https://github.com/lazyxeon/AstraWeave/commit/38c585b7b)).
- Phase 1.6-F.2-T.C: add highland-Y-max regression test + final continental_min tune ([`ff7bc3cd9`](https://github.com/lazyxeon/AstraWeave/commit/ff7bc3cd9)).
- 2 `Phase 1.6-F.2-T.D` commits on `main`, newest "Phase 1.6-F.2-T.D: fix §9 + §10 F.2-T.D hash references to 9ac2e5447" ([`c73219948`](https://github.com/lazyxeon/AstraWeave/commit/c73219948)) through oldest "Phase 1.6-F.2-T.D: close F.2 tuning pass + plan updates" ([`e304ae52e`](https://github.com/lazyxeon/AstraWeave/commit/e304ae52e)).
- Phase 1.6-F.2.A: extend BiomeNoisePreset + NoiseConfig with noise_type and continental fields ([`e1f7f29db`](https://github.com/lazyxeon/AstraWeave/commit/e1f7f29db)).
- Phase 1.6-F.2.B: enable DomainWarpedNoise + continental modulation for five presets ([`b91cc6a6e`](https://github.com/lazyxeon/AstraWeave/commit/b91cc6a6e)).
- Phase 1.6-F.2.C: continental-scale mountain-amplitude modulation in TerrainNoise ([`b566bbe4a`](https://github.com/lazyxeon/AstraWeave/commit/b566bbe4a)).
- 2 `Phase 1.6-F.2.D` commits on `main`, newest "Phase 1.6-F.2.D: fix §9 F.2.D commit hash to 3242f5302" ([`7734bbf61`](https://github.com/lazyxeon/AstraWeave/commit/7734bbf61)) through oldest "Phase 1.6-F.2.D: close F.2" ([`f9f18747b`](https://github.com/lazyxeon/AstraWeave/commit/f9f18747b)).
- Phase 1.6-F.3-phase-0.A: AdvancedErosionSimulator static audit ([`1f12aae85`](https://github.com/lazyxeon/AstraWeave/commit/1f12aae85)).
- Phase 1.6-F.3-phase-0.B: synthetic heightmap behavioral tests for AdvancedErosionSimulator ([`dcc3f546b`](https://github.com/lazyxeon/AstraWeave/commit/dcc3f546b)).
- Phase 1.6-F.3-phase-0.C: AdvancedErosionSimulator performance characterization ([`d522eff1a`](https://github.com/lazyxeon/AstraWeave/commit/d522eff1a)).
- 2 `Phase 1.6-F.3-phase-0.E` commits on `main`, newest "Phase 1.6-F.3-phase-0.E: fix §9 + §10 F.3-phase-0.E hash references to a9209100d" ([`2fb927b31`](https://github.com/lazyxeon/AstraWeave/commit/2fb927b31)) through oldest "Phase 1.6-F.3-phase-0.E: close F.3-phase-0 soundness audit" ([`fcf7ddb65`](https://github.com/lazyxeon/AstraWeave/commit/fcf7ddb65)).
- Phase 1.6-F.3-phase-1.A+B: biome-weight restructure + halo scaffolding ([`92f9466ac`](https://github.com/lazyxeon/AstraWeave/commit/92f9466ac)).
- 2 `Phase 1.6-F.3-phase-1.C` commits on `main`, newest "Phase 1.6-F.3-phase-1.C: fix §9 + §10 F.3-phase-1.C hash references to 86d6661cf" ([`f87aeb23e`](https://github.com/lazyxeon/AstraWeave/commit/f87aeb23e)) through oldest "Phase 1.6-F.3-phase-1.C: close F.3-phase-1 scaffolding" ([`b9ccf7525`](https://github.com/lazyxeon/AstraWeave/commit/b9ccf7525)).
- Phase 1.6-F.3-phase-2.A: climate → ErosionPreset mapping helper ([`4a86f59d5`](https://github.com/lazyxeon/AstraWeave/commit/4a86f59d5)).
- Phase 1.6-F.3-phase-2.B: balanced droplet-count preset variants ([`f215bf908`](https://github.com/lazyxeon/AstraWeave/commit/f215bf908)).
- Phase 1.6-F.3-phase-2.C: wire AdvancedErosionSimulator with per-climate presets ([`f037bdd7e`](https://github.com/lazyxeon/AstraWeave/commit/f037bdd7e)).
- Phase 1.6-F.3-phase-2.D: chunk-boundary continuity + biome-weight stability tests ([`6304e3935`](https://github.com/lazyxeon/AstraWeave/commit/6304e3935)).
- Phase 1.6-F.3-phase-2.E: performance characterization (rayon deferred) ([`d6bb0d12e`](https://github.com/lazyxeon/AstraWeave/commit/d6bb0d12e)).
- 2 `Phase 1.6-F.3-phase-2.F` commits on `main`, newest "Phase 1.6-F.3-phase-2.F: fix §9 + §10 F.3-phase-2.F hash references to 0693d4c18" ([`af171b97a`](https://github.com/lazyxeon/AstraWeave/commit/af171b97a)) through oldest "Phase 1.6-F.3-phase-2.F: close F.3-phase-2 + F.3 overall" ([`0602319d8`](https://github.com/lazyxeon/AstraWeave/commit/0602319d8)).
- Phase 1.6-F.3-phase-3.A: stitching + scale diagnostic ([`2474440a9`](https://github.com/lazyxeon/AstraWeave/commit/2474440a9)).
- Phase 1.6-F.3-phase-3.B: seamless erosion research + simulator readiness audit ([`f39ac5c5b`](https://github.com/lazyxeon/AstraWeave/commit/f39ac5c5b)).
- Phase 1.6-F.3-phase-3.C: world-coordinate droplet seeding for seamless chunk boundaries ([`3657707dc`](https://github.com/lazyxeon/AstraWeave/commit/3657707dc)).
- 2 `Phase 1.6-F.3-phase-3.E` commits on `main`, newest "Phase 1.6-F.3-phase-3.E: fix §9 + §10 F.3-phase-3.E hash references to dc1cc19f1" ([`40699f3df`](https://github.com/lazyxeon/AstraWeave/commit/40699f3df)) through oldest "Phase 1.6-F.3-phase-3.E: close F.3-phase-3" ([`4e990f5bb`](https://github.com/lazyxeon/AstraWeave/commit/4e990f5bb)).
- Phase 1.6-F.3-phase-4.A: Andrew-gate diagnostic + biome-weight edge invariant ([`1db98c24e`](https://github.com/lazyxeon/AstraWeave/commit/1db98c24e)).
- Phase 1.6-F.3-phase-4.B: shared-vertex averaging at chunk assembly ([`89ba78dfa`](https://github.com/lazyxeon/AstraWeave/commit/89ba78dfa)).
- Phase 1.6-F.3-phase-4.C: reduce balanced-preset droplet counts for scale recovery ([`c4beae326`](https://github.com/lazyxeon/AstraWeave/commit/c4beae326)).
- 2 `Phase 1.6-F.3-phase-4.D` commits on `main`, newest "Phase 1.6-F.3-phase-4.D: fix §9 + §10 F.3-phase-4.D hash references to 612a023ab" ([`8caf851e6`](https://github.com/lazyxeon/AstraWeave/commit/8caf851e6)) through oldest "Phase 1.6-F.3-phase-4.D: close F.3-phase-4 — F.3 complete" ([`1d1bbc861`](https://github.com/lazyxeon/AstraWeave/commit/1d1bbc861)).
- Phase 1.6-F.4.B.1: terrain scale diagnostic ([`a550db92d`](https://github.com/lazyxeon/AstraWeave/commit/a550db92d)).
- Phase 1.6-F.4.B.2.A: chunk extent 256→512 WU, vertex density 64→96 ([`741e3c192`](https://github.com/lazyxeon/AstraWeave/commit/741e3c192)).
- Phase 1.6-F.4.B.2.B: amplitude ×3-8 for Target B + Mountain Drama slider ([`810643f46`](https://github.com/lazyxeon/AstraWeave/commit/810643f46)).
- Phase 1.6-F.4.B.2.C: radius 5→10 world extent (Target B horizontal scale) ([`38a54ee6f`](https://github.com/lazyxeon/AstraWeave/commit/38a54ee6f)).
- Phase 1.6-F.4.B.2.D: rayon parallelization for chunk generation ([`667718ffb`](https://github.com/lazyxeon/AstraWeave/commit/667718ffb)).
- Phase 1.6-F.4.B.2.E: tree multiplier 14×→4× for realistic peak-to-tree ratio ([`dac4aa685`](https://github.com/lazyxeon/AstraWeave/commit/dac4aa685)).
- Phase 1.6-F.4.B.2.F: elevation bands + continental scale + F.2 regression recalibration ([`c62b990fb`](https://github.com/lazyxeon/AstraWeave/commit/c62b990fb)).
- 2 `Phase 1.6-F.4.B.2.G` commits on `main`, newest "Phase 1.6-F.4.B.2.G: fix §9 + §10 F.4.B.2.G hash references to f32f99c16" ([`f4193353a`](https://github.com/lazyxeon/AstraWeave/commit/f4193353a)) through oldest "Phase 1.6-F.4.B.2.G: close F.4.B.2 Target B implementation" ([`ff32f12b9`](https://github.com/lazyxeon/AstraWeave/commit/ff32f12b9)).
- Phase 1.6-F.4.B.2.H: fog recalibration for Target B world extent ([`022e618ab`](https://github.com/lazyxeon/AstraWeave/commit/022e618ab)).
- Phase 1.6-F.4.B.3.A: Uber Noise research pass ([`61e660307`](https://github.com/lazyxeon/AstraWeave/commit/61e660307)).
- Phase 1.6-F.4.B.3.B: octave-emphasis tuning for Mountain/Tundra presets ([`253aedc63`](https://github.com/lazyxeon/AstraWeave/commit/253aedc63)).
- 2 `Phase 1.6-F.4.B.3.B revert` commits on `main`, newest "Phase 1.6-F.4.B.3.B revert: fix §9 + §10 hash references to 29afe027e" ([`ec58d88a5`](https://github.com/lazyxeon/AstraWeave/commit/ec58d88a5)) through oldest "Phase 1.6-F.4.B.3.B revert: octave-emphasis ineffective" ([`cb8c01805`](https://github.com/lazyxeon/AstraWeave/commit/cb8c01805)).
- 2 `Phase 1.6-F.4.B.3.C` commits on `main`, newest "Phase 1.6-F.4.B.3.C: fix §9 + §10 hash references to adcd70331" ([`553ccd1e3`](https://github.com/lazyxeon/AstraWeave/commit/553ccd1e3)) through oldest "Phase 1.6-F.4.B.3.C: runevision filter integration for Mountain/Tundra presets" ([`6b81f8ec9`](https://github.com/lazyxeon/AstraWeave/commit/6b81f8ec9)).
- 2 `Phase 1.6-F.4.B.3.D.1` commits on `main`, newest "Phase 1.6-F.4.B.3.D.1: fix §9 + §10 hash references to 8f708ec12" ([`60308aace`](https://github.com/lazyxeon/AstraWeave/commit/60308aace)) through oldest "Phase 1.6-F.4.B.3.D.1: climate field architecture" ([`a8da51109`](https://github.com/lazyxeon/AstraWeave/commit/a8da51109)).
- 2 `Phase 1.6-F.4.B.3.D.2` commits on `main`, newest "Phase 1.6-F.4.B.3.D.2: fix §9 + §10 hash references to fa902089c" ([`80456c1dd`](https://github.com/lazyxeon/AstraWeave/commit/80456c1dd)) through oldest "Phase 1.6-F.4.B.3.D.2: Whittaker biome lookup" ([`964816c99`](https://github.com/lazyxeon/AstraWeave/commit/964816c99)).
- Phase 1.6-F.4.B.3.D.3: §9 + §10 update for D.3a/b/c ([`5da2cc75c`](https://github.com/lazyxeon/AstraWeave/commit/5da2cc75c)).
- Phase 1.6-F.4.B.3.D.3a: BiomeParameters struct + per-biome defaults ([`588c7e76b`](https://github.com/lazyxeon/AstraWeave/commit/588c7e76b)).
- Phase 1.6-F.4.B.3.D.3b: per-vertex biome lookup in WorldGenerator ([`14d74ab54`](https://github.com/lazyxeon/AstraWeave/commit/14d74ab54)).
- Phase 1.6-F.4.B.3.D.3c: remove legacy BiomeNoisePreset + retire preset-shaped tests ([`de1c85771`](https://github.com/lazyxeon/AstraWeave/commit/de1c85771)).
- 2 `Phase 1.6-F.4.B.3.D.4` commits on `main`, newest "Phase 1.6-F.4.B.3.D.4: §9 + §10 update for biome blending" ([`eb3c71e63`](https://github.com/lazyxeon/AstraWeave/commit/eb3c71e63)) through oldest "Phase 1.6-F.4.B.3.D.4: scattered-convolution biome blending (noiseposti.ng)" ([`df640a629`](https://github.com/lazyxeon/AstraWeave/commit/df640a629)).
- Phase 1.6-F.4.B.3.D.5: §9 + §10 doc update ([`f5cbb2bbd`](https://github.com/lazyxeon/AstraWeave/commit/f5cbb2bbd)).
- Phase 1.6-F.4.B.3.D.5-diagnostic-2: real-heightmap biome distribution measurement ([`a7ca9b2c7`](https://github.com/lazyxeon/AstraWeave/commit/a7ca9b2c7)).
- 2 `Phase 1.6-F.4.B.3.D.5-diagnostic-3` commits on `main`, newest "Phase 1.6-F.4.B.3.D.5-diagnostic-3: fix §10 hash reference to 4fe47ef47" ([`b0c0cf74e`](https://github.com/lazyxeon/AstraWeave/commit/b0c0cf74e)) through oldest "Phase 1.6-F.4.B.3.D.5-diagnostic-3: cross-archetype terrain similarity measurement" ([`09f2bf76b`](https://github.com/lazyxeon/AstraWeave/commit/09f2bf76b)).
- 2 `Phase 1.6-F.4.B.3.D.5-fix` commits on `main`, newest "Phase 1.6-F.4.B.3.D.5-fix: fix §9 + §10 hash references to 5a5ea6ea0" ([`ebe87a66c`](https://github.com/lazyxeon/AstraWeave/commit/ebe87a66c)) through oldest "Phase 1.6-F.4.B.3.D.5-fix: Path B amplitude reduction for elevation-overlay biomes" ([`d18aa2d0d`](https://github.com/lazyxeon/AstraWeave/commit/d18aa2d0d)).
- Phase 1.6-F.4.B.3.D.5a: define five additional WorldArchetype presets + WorldArchetypeId enum ([`a015704e0`](https://github.com/lazyxeon/AstraWeave/commit/a015704e0)).
- Phase 1.6-F.4.B.3.D.5b: replace Primary Biome dropdown with World Archetype dropdown; remove Mountain Drama slider ([`7c96c76df`](https://github.com/lazyxeon/AstraWeave/commit/7c96c76df)).
- Phase 1.6-I: heightmap generator investigation report ([`79505e0c0`](https://github.com/lazyxeon/AstraWeave/commit/79505e0c0)).
- Phase 1.A: enable terrain-splat-arrays feature by default ([`9c8226376`](https://github.com/lazyxeon/AstraWeave/commit/9c8226376)).
- Phase 1.B: construct EditorTerrainSplat field on EngineRenderAdapter ([`754fb431a`](https://github.com/lazyxeon/AstraWeave/commit/754fb431a)).
- Phase 1.C: wire splat builder + upload into upload_terrain_chunks ([`4b1042ab1`](https://github.com/lazyxeon/AstraWeave/commit/4b1042ab1)).
- Phase 1.D: author forward-lit splat shader pbr_terrain_forward.wgsl ([`572a98636`](https://github.com/lazyxeon/AstraWeave/commit/572a98636)).
- Phase 1.E.1.a: add CameraForwardGpu and TerrainSceneEnvGpu types ([`ffeb20eab`](https://github.com/lazyxeon/AstraWeave/commit/ffeb20eab)).
- Phase 1.E.1.b: add forward-pipeline BGLs, buffers, and bind groups ([`7b7b60d41`](https://github.com/lazyxeon/AstraWeave/commit/7b7b60d41)).
- Phase 1.E.1.c: implement ensure_forward_pipeline + shader composition ([`d786ba40c`](https://github.com/lazyxeon/AstraWeave/commit/d786ba40c)).
- Phase 1.E.2: add forward-path update/upload/draw methods to TerrainMaterialManager ([`fc4c310e7`](https://github.com/lazyxeon/AstraWeave/commit/fc4c310e7)).
- Phase 1.E.3: integrate terrain-forward state and draw block into Renderer::draw_into ([`44a7dc52f`](https://github.com/lazyxeon/AstraWeave/commit/44a7dc52f)).
- Phase 1.E.4.a: add biome placeholder texture module ([`45ad30d41`](https://github.com/lazyxeon/AstraWeave/commit/45ad30d41)).
- Phase 1.E.4.b: initialize terrain_forward and upload biome materials on project open ([`fa091fe69`](https://github.com/lazyxeon/AstraWeave/commit/fa091fe69)).
- Phase 1.E.4.c: route terrain chunks through Renderer::upload_terrain_chunk ([`152989ccb`](https://github.com/lazyxeon/AstraWeave/commit/152989ccb)).
- Phase 1.E.5: remove superseded terrain_splat field from EngineRenderAdapter ([`9c89e6bdd`](https://github.com/lazyxeon/AstraWeave/commit/9c89e6bdd)).
- Phase 1.F: Phase 1 COMPLETE — forward-lit splat pipeline active in editor ([`01fdfaf77`](https://github.com/lazyxeon/AstraWeave/commit/01fdfaf77)).
- 2 `Phase 1.X-design` commits on `main`, newest "Phase 1.X-design: fix hash references to 3e9316e7a" ([`038878465`](https://github.com/lazyxeon/AstraWeave/commit/038878465)) through oldest "Phase 1.X-design: regional archetype variation campaign ground-truth doc" ([`7a85fe96c`](https://github.com/lazyxeon/AstraWeave/commit/7a85fe96c)).
- Phase 1.X-F.1.A: PvFold helper + spline types ([`33115257e`](https://github.com/lazyxeon/AstraWeave/commit/33115257e)).
- Phase 1.X-F.1.B: extend ClimateSample with erosion + weirdness ([`cf0f00ec6`](https://github.com/lazyxeon/AstraWeave/commit/cf0f00ec6)).
- Phase 1.X-F.1.C: F.1 unit tests + regression verification ([`ebc0421f6`](https://github.com/lazyxeon/AstraWeave/commit/ebc0421f6)).
- 2 `Phase 1.X-F.1.D` commits on `main`, newest "Phase 1.X-F.1.D: hash-fixup F.1.D commit hash to c581f6d68" ([`ed4c97e61`](https://github.com/lazyxeon/AstraWeave/commit/ed4c97e61)) through oldest "Phase 1.X-F.1.D: close F.1" ([`21bb737aa`](https://github.com/lazyxeon/AstraWeave/commit/21bb737aa)).
- Phase 1.X-F.2.A: Spline1D + ParamSpline infrastructure ([`b9422ee50`](https://github.com/lazyxeon/AstraWeave/commit/b9422ee50)).
- Phase 1.X-F.2.B: BootstrapSplineSet + 6 archetype defaults ([`be117bc53`](https://github.com/lazyxeon/AstraWeave/commit/be117bc53)).
- Phase 1.X-F.2.C: extend WorldArchetype with bootstrap_splines ([`a82c1b2ac`](https://github.com/lazyxeon/AstraWeave/commit/a82c1b2ac)).
- 2 `Phase 1.X-F.2.D` commits on `main`, newest "Phase 1.X-F.2.D: hash-fixup F.2.D commit hash to 6bb234c0b" ([`0a285adb8`](https://github.com/lazyxeon/AstraWeave/commit/0a285adb8)) through oldest "Phase 1.X-F.2.D: close F.2" ([`5279b9e07`](https://github.com/lazyxeon/AstraWeave/commit/5279b9e07)).
- Phase 1.X-F.3.A: sample_height_with_params method ([`6794da091`](https://github.com/lazyxeon/AstraWeave/commit/6794da091)).
- Phase 1.X-F.3.B: wire BootstrapSplineSet (Continental Temperate) ([`b91e6d363`](https://github.com/lazyxeon/AstraWeave/commit/b91e6d363)).
- Phase 1.X-F.3.C: F.3 regression + smoke tests ([`67b8ef714`](https://github.com/lazyxeon/AstraWeave/commit/67b8ef714)).
- 2 `Phase 1.X-research` commits on `main`, newest "Phase 1.X-research: fix §10 hash reference to 5fd98a9e5" ([`ebfc7b273`](https://github.com/lazyxeon/AstraWeave/commit/ebfc7b273)) through oldest "Phase 1.X-research: regional archetype variation research audit" ([`2087d1bfb`](https://github.com/lazyxeon/AstraWeave/commit/2087d1bfb)).
- Refactor editor subsystems: extract docking synchronization, hotkey handling, and scene statistics into separate modules ([`a2ca98997`](https://github.com/lazyxeon/AstraWeave/commit/a2ca98997)).
- remove(ecs): delete ParallelSchedule; document engine as single-threaded ECS ([`3c852c7b1`](https://github.com/lazyxeon/AstraWeave/commit/3c852c7b1)).
- style: fix formatting across all workspace crates for CI compliance ([`6e70126c6`](https://github.com/lazyxeon/AstraWeave/commit/6e70126c6)).

### 2026-03

#### Changed
- 5 `astraweave-ai` commits on `main`, newest "astraweave-ai: add 4 mutation-killing tests for veilweaver.rs" ([`2b00439e8`](https://github.com/lazyxeon/AstraWeave/commit/2b00439e8)) through oldest "astraweave-ai: add 10 mutation-killing tests for tool_sandbox.rs" ([`be240ccf5`](https://github.com/lazyxeon/AstraWeave/commit/be240ccf5)).
- 2 `astraweave-ai/goap` commits on `main`, newest "astraweave-ai/goap: add 6 more mutation-killing tests (round 2)" ([`83ea239c4`](https://github.com/lazyxeon/AstraWeave/commit/83ea239c4)) through oldest "astraweave-ai/goap: add 35 mutation-killing tests + fix unrevereted mutant" ([`d8cb5bb35`](https://github.com/lazyxeon/AstraWeave/commit/d8cb5bb35)).
- astraweave-ai/goap batch 2: add mutation-killing tests for 6 files ([`4803b65c5`](https://github.com/lazyxeon/AstraWeave/commit/4803b65c5)).
- astraweave-ai/goap batch 2 round 2: targeted mutation-killing tests ([`a25ad7598`](https://github.com/lazyxeon/AstraWeave/commit/a25ad7598)).
- astraweave-ai/goap batch 3a: add mutation-killing tests for planner, state, history, plan_visualizer ([`354230274`](https://github.com/lazyxeon/AstraWeave/commit/354230274)).
- astraweave-core: add mutation-killing tests for ecs_adapter (cooldown decay, sync) ([`d2f16c038`](https://github.com/lazyxeon/AstraWeave/commit/d2f16c038)).
- 5 `astraweave-fluids` commits on `main`, newest "astraweave-fluids: replace caustics golden test with multi-point + chromatic mutation killers" ([`891d27787`](https://github.com/lazyxeon/AstraWeave/commit/891d27787)) through oldest "astraweave-fluids: add mutation-killing test for emitter jitter+velocity calculations" ([`337cf8d88`](https://github.com/lazyxeon/AstraWeave/commit/337cf8d88)).
- astraweave-memory: 100% adjusted mutation kill rate (890/1036, 56 equiv/non-testable) ([`d17eac091`](https://github.com/lazyxeon/AstraWeave/commit/d17eac091)).
- 3 `astraweave-sdk` commits on `main`, newest "astraweave-sdk: strengthen removal detection test to check exact entity IDs" ([`3dbf7b6be`](https://github.com/lazyxeon/AstraWeave/commit/3dbf7b6be)) through oldest "astraweave-sdk: add mutation-killing tests for destroy, delta, write_cstr, current_map" ([`f40c6b281`](https://github.com/lazyxeon/AstraWeave/commit/f40c6b281)).
- astraweave-ui: 100% adjusted mutation kill rate (507/999, 492 equiv/non-testable) ([`c7e7b41fb`](https://github.com/lazyxeon/AstraWeave/commit/c7e7b41fb)).
- 5 `chore` commits on `main`, newest "chore: remove outdated entry for astraweave-ai from low test density list" ([`d74654c05`](https://github.com/lazyxeon/AstraWeave/commit/d74654c05)) through oldest "chore: checkpoint before batch 3a mutation re-run" ([`8affc6cd5`](https://github.com/lazyxeon/AstraWeave/commit/8affc6cd5)).
- 3 `ci(docs)` commits on `main`, newest "ci(docs): fix mdBook version to 0.5.2, correct mermaid download URL" ([`80892dad1`](https://github.com/lazyxeon/AstraWeave/commit/80892dad1)) through oldest "ci(docs): deploy on workflow_dispatch, not only on push to main" ([`96887826d`](https://github.com/lazyxeon/AstraWeave/commit/96887826d)).
- docs: update mutation testing audit v1.4.0 — astraweave-fluids complete (100% adjusted) ([`e4b62cd8f`](https://github.com/lazyxeon/AstraWeave/commit/e4b62cd8f)).
- 5 `feat` commits on `main`, newest "feat: add BlueprintOverlay for 3D zone visualization in viewport" ([`628abf31a`](https://github.com/lazyxeon/AstraWeave/commit/628abf31a)) through oldest "feat: restyle benchmark dashboard to match landing page" ([`bebd87e06`](https://github.com/lazyxeon/AstraWeave/commit/bebd87e06)).
- feat(ecs): add capacity accessors and mutation-killing tests for reserve/with_capacity ([`fb16f150d`](https://github.com/lazyxeon/AstraWeave/commit/fb16f150d)).
- feat(tests): add integration tests for && → || mutation kills in dialogue and cinematic logic ([`f474b6697`](https://github.com/lazyxeon/AstraWeave/commit/f474b6697)).
- feat(weaving): mutation testing complete — 99.2% adjusted kill rate, 182 new tests ([`f910acf34`](https://github.com/lazyxeon/AstraWeave/commit/f910acf34)).
- 7 `fix` commits on `main`, newest "fix: remove blank line breaking hero HTML block in mdBook" ([`a86c30cd3`](https://github.com/lazyxeon/AstraWeave/commit/a86c30cd3)) through oldest "fix: repair corrupted sanitizers.yml and restore World::get() alive check" ([`b7cd0a2d3`](https://github.com/lazyxeon/AstraWeave/commit/b7cd0a2d3)).
- fix(astraweave-prompts): fix render counter artifact + complete mutation audit ([`392445b66`](https://github.com/lazyxeon/AstraWeave/commit/392445b66)).
- 2 `fix(docs)` commits on `main`, newest "fix(docs): fetch LFS images via media CDN instead of full lfs checkout" ([`9ada02a3c`](https://github.com/lazyxeon/AstraWeave/commit/9ada02a3c)) through oldest "fix(docs): use checkout lfs:true instead of manual git lfs pull" ([`0071ed95c`](https://github.com/lazyxeon/AstraWeave/commit/0071ed95c)).
- 2 `fix(render_metrics)` commits on `main`, newest "fix(render_metrics): correct cache effectiveness check and render summary logic" ([`cfddd82aa`](https://github.com/lazyxeon/AstraWeave/commit/cfddd82aa)) through oldest "fix(render_metrics): update success_rate calculation and cache effectiveness check" ([`2387a3fc1`](https://github.com/lazyxeon/AstraWeave/commit/2387a3fc1)).
- fix(trigger_contains): correct distance comparison logic for trigger zones ([`810deb25b`](https://github.com/lazyxeon/AstraWeave/commit/810deb25b)).
- 16 `misc` commits on `main`, newest "Refactor code structure for improved readability and maintainability" ([`192bd243e`](https://github.com/lazyxeon/AstraWeave/commit/192bd243e)) through oldest "Add comprehensive mutation-killing tests for adapter, plan_analyzer, goal_validator (88+139+96 misses)" ([`e175cd581`](https://github.com/lazyxeon/AstraWeave/commit/e175cd581)).
- pre-mutation checkpoint: astraweave-ai baseline (268 tests) ([`71cbde5a4`](https://github.com/lazyxeon/AstraWeave/commit/71cbde5a4)).
- 2 `Pre-mutation checkpoint` commits on `main`, newest "Pre-mutation checkpoint: astraweave-fluids baseline" ([`b597d3b3a`](https://github.com/lazyxeon/AstraWeave/commit/b597d3b3a)) through oldest "Pre-mutation checkpoint: astraweave-core baseline" ([`bc77e25aa`](https://github.com/lazyxeon/AstraWeave/commit/bc77e25aa)).
- style(dashboard): restyle interactive benchmark dashboard to AstraWeave design language ([`9fdf376bd`](https://github.com/lazyxeon/AstraWeave/commit/9fdf376bd)).
- style(landing): replace matplotlib benchmark chart with dark-themed inline SVG ([`0829f0633`](https://github.com/lazyxeon/AstraWeave/commit/0829f0633)).
- 2 `test` commits on `main`, newest "test: COMPLETE mutation audit - all 59 library crates verified (v1.42.0)" ([`edac4e651`](https://github.com/lazyxeon/AstraWeave/commit/edac4e651)) through oldest "test: complete mutation audit for 6 remaining crates (48->54/59)" ([`f1493f5e0`](https://github.com/lazyxeon/AstraWeave/commit/f1493f5e0)).
- test(astract): complete mutation audit (67.1% raw, 100% adj) ([`e840bc55c`](https://github.com/lazyxeon/AstraWeave/commit/e840bc55c)).
- test(astraweave-asset): complete mutation audit (42.1% raw, 100% adj) ([`5c5e79782`](https://github.com/lazyxeon/AstraWeave/commit/5c5e79782)).
- test(astraweave-audio): complete mutation audit (49.5% raw, 100% adj) ([`2f3202035`](https://github.com/lazyxeon/AstraWeave/commit/2f3202035)).
- test(astraweave-blend): complete mutation audit (46.0% raw, 100% adj) ([`4418ad6ee`](https://github.com/lazyxeon/AstraWeave/commit/4418ad6ee)).
- test(astraweave-cinematics): complete mutation audit (99.12% raw, 100% adj) ([`9ad3eeada`](https://github.com/lazyxeon/AstraWeave/commit/9ad3eeada)).
- test(astraweave-context): complete mutation audit (76.5% raw, 100% adj) ([`fbe5637bd`](https://github.com/lazyxeon/AstraWeave/commit/fbe5637bd)).
- test(astraweave-dialogue): complete mutation audit (92.5% raw, 100% adj) ([`21014312b`](https://github.com/lazyxeon/AstraWeave/commit/21014312b)).
- test(astraweave-director): complete mutation audit (65.9% raw, 100% adj) ([`0c92d0012`](https://github.com/lazyxeon/AstraWeave/commit/0c92d0012)).
- test(astraweave-embeddings): complete mutation audit (52.3% raw, 100% adj) ([`9a1ecdd9a`](https://github.com/lazyxeon/AstraWeave/commit/9a1ecdd9a)).
- test(astraweave-input): complete mutation audit (90.99% raw, 100% adj) ([`2f0afba91`](https://github.com/lazyxeon/AstraWeave/commit/2f0afba91)).
- test(astraweave-ipc): complete mutation audit (100% raw, 100% adj) ([`ca6175833`](https://github.com/lazyxeon/AstraWeave/commit/ca6175833)).
- test(astraweave-llm): complete mutation audit (59.4% raw, 100% adj) ([`d4279d53a`](https://github.com/lazyxeon/AstraWeave/commit/d4279d53a)).
- test(astraweave-llm-eval): complete mutation audit (30.0% raw, 100% adj) ([`5b40ac8ca`](https://github.com/lazyxeon/AstraWeave/commit/5b40ac8ca)).
- test(astraweave-materials): complete mutation audit (67.5% raw, 100% adj) ([`65719da30`](https://github.com/lazyxeon/AstraWeave/commit/65719da30)).
- test(astraweave-npc): complete mutation audit (35.8% raw, 100% adj) ([`3964fdab0`](https://github.com/lazyxeon/AstraWeave/commit/3964fdab0)).
- test(astraweave-observability): complete mutation audit (29.2% raw, 100% adj) ([`d296ea5fe`](https://github.com/lazyxeon/AstraWeave/commit/d296ea5fe)).
- test(astraweave-optimization): complete mutation audit (5.1% raw, 100% adj) ([`46b578a62`](https://github.com/lazyxeon/AstraWeave/commit/46b578a62)).
- test(astraweave-pcg): complete mutation audit (65.3% raw, 100% adj) ([`8a00d5338`](https://github.com/lazyxeon/AstraWeave/commit/8a00d5338)).
- test(astraweave-persistence-ecs): complete mutation audit (47.6% raw, 100% adj) ([`7cca6a4ed`](https://github.com/lazyxeon/AstraWeave/commit/7cca6a4ed)).
- test(astraweave-persona): complete mutation audit (76.2% raw, 100% adj) ([`f11e50651`](https://github.com/lazyxeon/AstraWeave/commit/f11e50651)).
- test(astraweave-quests): complete mutation audit (66.5% raw, 100% adj) ([`ef61066aa`](https://github.com/lazyxeon/AstraWeave/commit/ef61066aa)).
- test(astraweave-rag): complete mutation audit (81.3% raw, 100% adj) ([`36f7efbce`](https://github.com/lazyxeon/AstraWeave/commit/36f7efbce)).
- test(astraweave-secrets): complete mutation audit (56.3% raw, 100% adj) ([`e042ccacc`](https://github.com/lazyxeon/AstraWeave/commit/e042ccacc)).
- test(veilweaver_slice_runtime): complete mutation testing audit + artifact fixes ([`f08e5e91c`](https://github.com/lazyxeon/AstraWeave/commit/f08e5e91c)).
- Update mutation testing audit: Phase 1 complete (9 crates, all >=96% raw kill rate) ([`79e0c4a80`](https://github.com/lazyxeon/AstraWeave/commit/79e0c4a80)).
- Update MUTATION_TESTING_AUDIT.md v1.5.0: astraweave-ai complete (100% adjusted, 29 files, ~1900 mutants) ([`1d5a7c3bf`](https://github.com/lazyxeon/AstraWeave/commit/1d5a7c3bf)).

### 2026-02

#### Changed
- docs: Update README crate structure to accurate 49-crate taxonomy with research-grade descriptions ([#174](https://github.com/lazyxeon/AstraWeave/pull/174)).

- Add performance documentation: benchmarks, budgets, methodology, and optimization guide ([`c05c9e793`](https://github.com/lazyxeon/AstraWeave/commit/c05c9e793)).
- asset system cleanup: removed incomplete/ broken textures and assets. added python materials script. ([`aaae44f84`](https://github.com/lazyxeon/AstraWeave/commit/aaae44f84)).
- 3 `ci` commits on `main`, newest "ci: add source-integrity gate to sanitizers workflow" ([`90028f358`](https://github.com/lazyxeon/AstraWeave/commit/90028f358)) through oldest "ci: Update checkout step to use LFS for glTF/glb fixtures" ([`c2fd5a9a4`](https://github.com/lazyxeon/AstraWeave/commit/c2fd5a9a4)).
- 10 `docs` commits on `main`, newest "docs: update MASTER_COVERAGE_REPORT v4.2.0 with Wave 2 mutation testing results" ([`b49260b82`](https://github.com/lazyxeon/AstraWeave/commit/b49260b82)) through oldest "docs: add mutation testing hit list for P0 crates - 11/13 complete" ([`f3462826f`](https://github.com/lazyxeon/AstraWeave/commit/f3462826f)).
- 9 `feat` commits on `main`, newest "feat: enhance sanitizer workflows and add TSan suppressions for improved testing reliability" ([`a7f8535ff`](https://github.com/lazyxeon/AstraWeave/commit/a7f8535ff)) through oldest "feat: Update benchmark dashboard to v7.0 with enhanced UI and new features" ([`b42421389`](https://github.com/lazyxeon/AstraWeave/commit/b42421389)).
- feat(ecs): add mutation resistance tests (~120 tests targeting all subsystems) ([`82cd42c09`](https://github.com/lazyxeon/AstraWeave/commit/82cd42c09)).
- 2 `feat(mutation)` commits on `main`, newest "feat(mutation): add sweep automation scripts, update Wave 2 plan to v1.1" ([`811e43d50`](https://github.com/lazyxeon/AstraWeave/commit/811e43d50)) through oldest "feat(mutation): Wave 2 editor panel coverage — 9 new test files (533 tests)" ([`ad3ef74ec`](https://github.com/lazyxeon/AstraWeave/commit/ad3ef74ec)).
- feat(physics): implement buoyancy force calculation in apply_buoyancy_forces ([`c87ddaa2b`](https://github.com/lazyxeon/AstraWeave/commit/c87ddaa2b)).
- 4 `feat(render)` commits on `main`, newest "feat(render): targeted projection mutation killers for clustered.rs:59-60" ([`47cb6a314`](https://github.com/lazyxeon/AstraWeave/commit/47cb6a314)) through oldest "feat(render): proactive Wave 2 remediation tests — environment, culling, LOD, animation, primitives" ([`f62bbd498`](https://github.com/lazyxeon/AstraWeave/commit/f62bbd498)).
- 2 `feat(terrain)` commits on `main`, newest "feat(terrain): add 59 heightmap remediation tests + fix sweep scripts" ([`90ea93ea4`](https://github.com/lazyxeon/AstraWeave/commit/90ea93ea4)) through oldest "feat(terrain): add 146 Wave 2 mutation remediation tests" ([`b508a2f83`](https://github.com/lazyxeon/AstraWeave/commit/b508a2f83)).
- 18 `fix` commits on `main`, newest "fix: update entity alive check comment to clarify cargo-mutants modification" ([`71c00f850`](https://github.com/lazyxeon/AstraWeave/commit/71c00f850)) through oldest "fix: restore has_offensive implementation corrupted by cargo-mutants" ([`ad26d3c41`](https://github.com/lazyxeon/AstraWeave/commit/ad26d3c41)).
- 7 `fix(aabb)` commits on `main`, newest "fix(aabb): correct half-extents calculation in AABB method for accurate dimensions" ([`0b4c49de2`](https://github.com/lazyxeon/AstraWeave/commit/0b4c49de2)) through oldest "fix(aabb): update intersection logic for accurate collision detection" ([`18ac32591`](https://github.com/lazyxeon/AstraWeave/commit/18ac32591)).
- 3 `fix(async_scheduler)` commits on `main`, newest "fix(async_scheduler): implement telemetry export to JSON file in export_telemetry method fix(spatial_hash): correct AABB min calculation in from_center_extents method" ([`c4ac533b1`](https://github.com/lazyxeon/AstraWeave/commit/c4ac533b1)) through oldest "fix(async_scheduler): update get_last_profile to return last_profile instead of default values" ([`42db06064`](https://github.com/lazyxeon/AstraWeave/commit/42db06064)).
- fix(capture_replay): update WorldSerde conversion to sort obstacles for consistent state capture fix(ecs_adapter): correct cooldown decay calculation to increment instead of decrement ([`09bae8740`](https://github.com/lazyxeon/AstraWeave/commit/09bae8740)).
- 17 `fix(physics)` commits on `main`, newest "fix(physics): correct calculation logic in PhysicsStepProfile for narrow and integration phase durations" ([`76fae3678`](https://github.com/lazyxeon/AstraWeave/commit/76fae3678)) through oldest "fix(physics): update body ID retrieval and tagging logic in PhysicsWorld" ([`4e55fb632`](https://github.com/lazyxeon/AstraWeave/commit/4e55fb632)).
- 3 `fix(spatial_hash)` commits on `main`, newest "fix(spatial_hash): correct world_to_cell method for accurate grid cell calculations" ([`2375c93ad`](https://github.com/lazyxeon/AstraWeave/commit/2375c93ad)) through oldest "fix(spatial_hash): correct AABB max calculation and update intersection logic" ([`0502feac5`](https://github.com/lazyxeon/AstraWeave/commit/0502feac5)).
- 39 `misc` commits on `main`, newest "Add mutation testing audit and Qwen3 latency optimization report" ([`ce9eb80c2`](https://github.com/lazyxeon/AstraWeave/commit/ce9eb80c2)) through oldest "Add comprehensive mutation-resistant tests for Toast UI system" ([`ec75038ad`](https://github.com/lazyxeon/AstraWeave/commit/ec75038ad)).
- quality: doc-tests, unsafe audit, fmt, dep hygiene, forbid(unsafe_code) ([`e48c3d6c8`](https://github.com/lazyxeon/AstraWeave/commit/e48c3d6c8)).
- 24 `test` commits on `main`, newest "test: add round-2 mutation kill tests for ECS subsystems" ([`a91a9d4c6`](https://github.com/lazyxeon/AstraWeave/commit/a91a9d4c6)) through oldest "test: add 45 edge-case tests across pcg/scene; fix 3 cargo-mutants leftovers" ([`0dcc91eec`](https://github.com/lazyxeon/AstraWeave/commit/0dcc91eec)).
- test(coverage): add 120+ tests across persona, scene, coordination, director crates ([`6d915b646`](https://github.com/lazyxeon/AstraWeave/commit/6d915b646)).
- 3 `test(render)` commits on `main`, newest "test(render): 114 proactive tests — asset_index, camera, instancing, biome_material" ([`86ad224d9`](https://github.com/lazyxeon/AstraWeave/commit/86ad224d9)) through oldest "test(render): add 92 remediation tests for types/culling/primitives/weather [wave2]" ([`fa1a583bc`](https://github.com/lazyxeon/AstraWeave/commit/fa1a583bc)).
- 4 `test(terrain)` commits on `main`, newest "test(terrain): wave2 shard20 remediation — 33 tests for 56 MISSED mutations" ([`40cd1cc23`](https://github.com/lazyxeon/AstraWeave/commit/40cd1cc23)) through oldest "test(terrain): add 25 golden-value tests for generate_indices + smooth [wave2]" ([`70a0b00ff`](https://github.com/lazyxeon/AstraWeave/commit/70a0b00ff)).
- Wave 2: proactive remediation - structures(61), streaming_diagnostics(44), biome_transition+env(67), clustered(16) = 188 new tests ([`8a0173421`](https://github.com/lazyxeon/AstraWeave/commit/8a0173421)).
- 17 `wave2` commits on `main`, newest "wave2: batch 12 — terrain renderer, debug quad, mesh registry, overlay, weather, clustered forward (39 tests)" ([`3a3f45b07`](https://github.com/lazyxeon/AstraWeave/commit/3a3f45b07)) through oldest "wave2: 26 lib.rs remediation tests + scatter test speed optimization (300s -> 2s)" ([`4275a4afd`](https://github.com/lazyxeon/AstraWeave/commit/4275a4afd)).

### 2026-01

#### Changed
- Editor stability phase1 ([#162](https://github.com/lazyxeon/AstraWeave/pull/162)).
- Merge/editor stability phase1 to main ([#166](https://github.com/lazyxeon/AstraWeave/pull/166)).
- chore: comprehensive root-level cleanup ([#168](https://github.com/lazyxeon/AstraWeave/pull/168)).
- Chore/root level cleanup ([#169](https://github.com/lazyxeon/AstraWeave/pull/169)).

- 2 `chore` commits on `main`, newest "chore: comprehensive clippy cleanup with -D warnings for production quality" ([`5cc8f4960`](https://github.com/lazyxeon/AstraWeave/commit/5cc8f4960)) through oldest "chore: fix clippy warnings across workspace" ([`6348df7ab`](https://github.com/lazyxeon/AstraWeave/commit/6348df7ab)).
- 2 `ci` commits on `main`, newest "ci: make PBR Pipeline and Documentation workflows fully non-blocking" ([`7af0fc6ae`](https://github.com/lazyxeon/AstraWeave/commit/7af0fc6ae)) through oldest "ci: fix GitHub Actions workflows to be non-blocking" ([`19a0a62d3`](https://github.com/lazyxeon/AstraWeave/commit/19a0a62d3)).
- docs: Update WORLD_CLASS_EDITOR_PLAN with completed infrastructure work ([`50394b533`](https://github.com/lazyxeon/AstraWeave/commit/50394b533)).
- 3 `docs(editor)` commits on `main`, newest "docs(editor): Update roadmap - Week 4 COMPLETE" ([`2c9ff971e`](https://github.com/lazyxeon/AstraWeave/commit/2c9ff971e)) through oldest "docs(editor): Update progress log in WORLD_CLASS_EDITOR_PLAN.md - Document completed Week 1-3 milestones - List session commits (4 feature commits) - Update status to Week 3 Complete - Note next steps (Week 4, texture loading, material editing)" ([`14b976ee4`](https://github.com/lazyxeon/AstraWeave/commit/14b976ee4)).
- 17 `feat(editor)` commits on `main`, newest "feat(editor): Week 7 Day 5 - Crash Recovery System" ([`19fc991a0`](https://github.com/lazyxeon/AstraWeave/commit/19fc991a0)) through oldest "feat(editor): Enhanced Debug menu with glTF model testing and diagnostics" ([`667958431`](https://github.com/lazyxeon/AstraWeave/commit/667958431)).
- 5 `fix` commits on `main`, newest "fix: resolve failing doc tests across workspace" ([`69c47018b`](https://github.com/lazyxeon/AstraWeave/commit/69c47018b)) through oldest "fix: revert CpuLight test to match current struct definition" ([`dec42d7aa`](https://github.com/lazyxeon/AstraWeave/commit/dec42d7aa)).
- 2 `misc` commits on `main`, newest "Add Ready Asset System with comprehensive asset readiness checks and UI panel" ([`d47f20b5a`](https://github.com/lazyxeon/AstraWeave/commit/d47f20b5a)) through oldest "Refactor code structure for improved readability and maintainability" ([`6b4b5cb1e`](https://github.com/lazyxeon/AstraWeave/commit/6b4b5cb1e)).
- Phase 10A: Fluids system enhancements, mutation testing infrastructure, editor panels, and water system improvements ([`c8d10c90e`](https://github.com/lazyxeon/AstraWeave/commit/c8d10c90e)).
- 3 `test` commits on `main`, newest "test: mark rule_orchestrator_performance test as ignored" ([`90aa9d3b1`](https://github.com/lazyxeon/AstraWeave/commit/90aa9d3b1)) through oldest "test: mark flaky/performance-sensitive tests as ignored" ([`e79430020`](https://github.com/lazyxeon/AstraWeave/commit/e79430020)).

#### Fixed
- astraweave-rag: `get_cached_result` in `astraweave-rag/src/pipeline.rs` computes `should_remove` and drops the `cache.get` read guard before `cache.remove` ([#169](https://github.com/lazyxeon/AstraWeave/pull/169)).

## [0.4.0] - 2025-11-18

Archival note for the workspace version named in [SECURITY.md](SECURITY.md). There is no `v0.4.0` git tag. The supported line is current `main` at that version number. The paths below are in the tree at [`0c986d9fa`](https://github.com/lazyxeon/AstraWeave/commit/0c986d9fa) (2025-11-18).

### Added
- VXGI in `astraweave-render/src/gi/vxgi.rs` ([#112](https://github.com/lazyxeon/AstraWeave/pull/112)).
- MegaLights clustered light culling in `astraweave-render/src/clustered_megalights.rs` (`MegaLightsRenderer`) ([`ddefcd70a`](https://github.com/lazyxeon/AstraWeave/commit/ddefcd70a)).
- Nanite-inspired meshlet pipeline in `astraweave-render/src/nanite_render.rs` and `astraweave-asset/src/nanite_preprocess.rs` ([#113](https://github.com/lazyxeon/AstraWeave/pull/113)).
- ECS capture/replay module `astraweave-core/src/capture_replay.rs` ([`257357062`](https://github.com/lazyxeon/AstraWeave/commit/257357062)).

### Changed
- Audit reports moved under `docs/audits/` ([`d8cd5f027`](https://github.com/lazyxeon/AstraWeave/commit/d8cd5f027)).

## [0.3.0] - 2025-08-02 (archival — no release tag)

### Added
- LLM orchestration crate (`astraweave-llm`).
- Rhai-based scripting in `astraweave-scripting` (historical notes incorrectly said “Rhino”).

## [0.2.0] - 2025-04-11 (archival — no release tag)

### Added
- wgpu clustered forward rendering, asset streaming utilities, and benchmark harness scaffolding.

## [0.1.0] - 2024-12-01 (archival — no release tag)

### Added
- Initial public workspace: core ECS, navigation, physics, and AI behavior crates.

[Unreleased]: https://github.com/lazyxeon/AstraWeave/compare/0c986d9fa...HEAD

