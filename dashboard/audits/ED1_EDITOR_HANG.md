# ED-1 — Editor First-Launch Hang: Investigation Outcome

> **Beat:** ED-1 (editor health) · **Date:** 2026-07-22 · **Symptom:** cold `cargo editor` hangs indefinitely — process alive, no GUI, none of the normal early startup logs; Ctrl+C (`0xC000013A`) + immediate re-run → full normal startup. Intermittent, correlated with "first launch in a while".
> Evidence tiers: **built** / **run** / **verified**. Diagnose-before-fix discipline applied: two hypothesis classes were disproved by evidence before any code was written; **no behavioral fix shipped** (Phase-1.4 outcome: the hang did not reproduce as an indefinite block, and the evidence chain points off the exe entirely) — what shipped is the discriminating startup tracing that makes the next natural occurrence self-diagnosing.

## 1. Rule-out ledger (each with its evidence)

### 1.1 `.aw_editor.lock` blocking acquire — DISPROVED (code read)

The prime external suspect. The mechanism (`tools/aw_editor/src/main.rs:1311-1397`) is a **crash-recovery marker, not a lock**: `create_lock_file` is a plain `fs::File::create` + write of `pid/started/scene` (no OS lock is ever held); `remove_lock_file` deletes on clean exit; `check_for_crash_recovery` — called from `EditorApp::new` (`main.rs:2873`), i.e. **after** the window and wgpu exist — reads the file and at most shows an in-GUI recovery dialog or cleans up. **There is no acquire path that can block, and the whole surface runs post-window.** A stale marker cannot produce a pre-GUI hang; its actual effect is the (by-design) "Previous session may have crashed" console line / recovery dialog. The marker's repeated presence in this repo's status over the past week is fully explained by automation sessions hard-killing editor processes (this session's own verification kill left one too — removed at session end).
*(The AD.5 precedent the campaign record cites — `tools/xtask/src/fetch_assets.rs`'s fs2 lock with kill-9-stale-lock messaging — is a genuinely different mechanism: a real `try_lock`-style file lock on the asset-fetch path. It is not involved in editor startup.)*

### 1.2 Silent build-lock wait — DISPROVED (demonstrated)

Launched `cargo editor` while another `cargo check --workspace` held the build/package lock: cargo's **first stderr line within seconds** is `Blocking waiting for file lock on package cache` (run + verified, 2026-07-22). Lock contention — including rust-analyzer holding the target dir — is *noisy*, never silent. A contention hang would have shown that line in the recording.

### 1.3 Exe-side pre-log block — NO MECHANISM + measured fast

The first line a healthy exe emits is the assets-dir resolution (`aw_editor::viewport::types`, triggered by the window-icon load in `main()`), **~130-250 ms after spawn** (two verified runs, log timestamps vs spawn). The code before it: `init_editor_tracing` (in-memory sink + fmt layer + EnvFilter — read, no sockets/exporters), three local `create_dir_all`, two file-`exists` probes (`find_assets_dir`). Nothing in that window performs an operation that can block indefinitely. GPU/driver/winit stalls — the plausible external-handshake class — occur **after** the assets-dir and eframe log lines (order verified in captured startups), so they cannot produce the observed zero-log symptom.

## 2. Conviction (to the achievable standard)

**The "hang" is cargo's near-silent cold build/freshness phase — the editor exe had not spawned yet.** The chain:

1. Zero exe logs + the exe's first log firing within ~250 ms of spawn ⇒ the exe never spawned during the hang. The alive process was **cargo**.
2. That phase was measured at **1m 11s on a warm machine** this session (`Finished ... in 1m 11s`; the only mid-phase output was a rustc warning block — no `Compiling` lines, no `Finished`/`Running` until the end). Cold conditions — commits accumulated since the last build (many crates re-check) + cold FS cache + antivirus re-scanning the multi-GB `target/` — plausibly stretch this to many minutes of *complete* silence: "indefinite" at human patience scale.
3. **Why Ctrl+C + immediate re-run works** (the key reconciling fact): cargo persists completed crate checks/fingerprints incrementally — attempt #1's work survives its own Ctrl+C. Demonstrated within this session: first run's phase **1m 11s** → identical relaunch minutes later: **3.86 s** (`Finished ... in 3.86s` → `Running ...aw_editor.exe` → full startup). The re-run is fast *because* the "hung" first attempt did the work.
4. **Why cold/first-launch correlates:** that is precisely when the freshness phase is longest (new commits, cold caches, AV rescans).
5. `0xC000013A` = STATUS_CONTROL_C_EXIT — the Ctrl+C landing on the cargo process; consistent, non-discriminating.

**Honest limit:** an indefinite (never-completing) block was not reproduced — this machine cannot be made truly cold on demand (no reboot/AV-purge available to the session). If a future occurrence shows cargo's `Running \`...aw_editor.exe\`` line followed by silence **without** the new banner (§3), this conviction is falsified and the block is somewhere new — which is exactly what the shipped tracing exists to reveal.

## 3. What shipped (`main.rs`, permanent structured tracing — no debug residue)

Two `tracing::info!` stage lines in `main()` (convention-consistent `[AW]` prefix):

1. **Startup banner** — the first line the process ever emits, immediately after tracing init: `[AW] aw_editor starting (pid=…, cwd=…)`.
2. **Pre-window stage line** — immediately before `eframe::run_native`: `[AW] startup: pre-window init done in <elapsed> — entering eframe (event loop + window + wgpu next)`.

Verified live ordering (run + verified): banner → assets-dir resolution → pre-window line (109.3 ms) → eframe renderer line → wgpu adapter logs. Combined with the pre-existing logs, every future startup self-bisects from the console alone:

| Console shows | Block location |
|---|---|
| No `Running ...aw_editor.exe` from cargo | cargo build/freshness phase (§2 — wait it out once; watch for `Blocking waiting for file lock` = contention) |
| `Running` but no banner | exe blocked before tracing init (would be NEW information — capture per §4) |
| Banner but no pre-window line | dirs/icon/assets-dir window (all local I/O) |
| Pre-window line but no eframe/adapter lines | winit/Vulkan/driver init — the external-handshake class |
| Adapter line but no window | post-device `EditorApp::new` (audio bridge, catalogs — all logged) |

## 4. Capture procedure for the next natural occurrence (director)

1. **Don't Ctrl+C yet.** Note which lines from the §3 table are present.
2. Task Manager → is `aw_editor.exe` running under the console's `cargo.exe`? (No → cargo phase; give it one honest multi-minute wait — if it eventually prints `Finished`, §2 is confirmed at natural scale.)
3. If `aw_editor.exe` exists and its last console line bisects to a stage: Task Manager → right-click `aw_editor.exe` → **Create dump file**, attach the `.dmp` path + the console text to the report. (No procdump/cdb/windbg exists on this machine — checked.)
4. Then Ctrl+C and note whether the re-run is instant (cargo-phase signature) or also hangs (exe-stage signature).

## 5. weaving_playground wedge — verdict: UNRELATED (different failure phase)

The playground wedge (`TWR_WATER_RECON.md` §1.7): window **created and painted early frames**, then the main thread parked (~4 s total CPU, "Not Responding", logs empty because the demo emits nothing at that stage) — a **post-window** init block inside a running exe. The editor symptom is **pre-exe** (zero exe output, no window ever). Different process phase, no shared mechanism in evidence; the playground item stays open as its own demo-health entry. (If the editor's next captured occurrence lands in the §3 "pre-window line but no eframe" row, re-examine for a shared winit/Vulkan cause — that is the only overlap window.)

## 6. Verification & residue

- `cargo check -p aw_editor` clean (exit 0); `cargo check --workspace` clean (exit 0, run this session pre-change and the change is two log lines). Live launch verified the log ordering (§3); the verification instance was hard-stopped and its crash-marker removed — the tree and editor are in normal state for the in-progress T.W.1 gate.
- No test accompanies the change: it is observability-only (two info lines); the "fails on old code" concept does not apply to a discriminator whose purpose is diagnosing a non-reproduced condition (Phase-1.4 branch).
- Pre-existing residue observed, not touched (anti-drift): `astraweave-core/benches/full_game_loop.rs:220` fails to compile under `--all-targets` (missing new `Pose` fields — stale bench); the known deferred warning set (`fast_executor`, gizmo unused import, etc.).
- `.aw_editor.lock` semantics untouched (exonerated; anti-drift). One naming note for a future hygiene pass: the file is a crash *marker*, and its `.lock` name is what invited the external misdiagnosis — renaming to `.aw_editor.session` (with recovery-path update) would prevent the next one. Not done this beat (beyond convicted scope).
