---
schema_version: 1
trace_id: audio
title: "Audio"
description: "Audio"
primary_crate: astraweave-audio
domain: gameplay
lifecycle_status: active
integration_status: wired
owns: [astraweave-audio]
doc_version: "1.2"
last_verified_commit: a2474c5b7
---

# Architecture Trace: Audio

## Metadata

| Field | Value |
|---|---|
| **System name** | Audio |
| **Primary crates** | `astraweave-audio` (sole production crate); `tools/aw_editor` (editor integration via `audio_bridge.rs`); `astraweave-npc` (single direct consumer in runtime) |
| **Document version** | 1.2 |
| **Last verified against commit** | `a2474c5b7` |
| **Last verified date** | 2026-05-12 |
| **Revision history** | 1.2 (2026-05-12): Deep investigation pass. Closed factual portion of Open Question 4 (aspirational docs origin) — git log identifies commit `28bc94f21` "Create comprehensive bespoke wiki with 51-section documentation structure (#34)" authored by GitHub Copilot bot on 2025-09-08; resolution moved to §6. Enriched §11 Open Questions 2 (PanMode), 8 (hardcoded AudioStats), and 9 (veilweaver_demo) with comprehensive git/source evidence. Corrected §9 hot-paths bench-name reference (`listener_movement_*` exercises `tick`, not a separate `audio_tick_per_frame` bench). Added new cognitive trap in §6: `EmitterId` collision silently reuses the existing `SpatialSink` and overwrites its emitter position. Added new §11 Open Question on the `EmitterId` allocator-less design.<br><br>1.1 (2026-05-12): Verification pass. Confirmed rodio→cpal `!Send + !Sync` chain via `cpal-0.15.3/src/platform/mod.rs:73, 736` (`NotSendSyncAcrossAllPlatforms(PhantomData<*mut ()>)` marker). Corrected Sink/SpatialSink Send/Sync claim — only `OutputStream` is the `!Send` blocker, not the sinks themselves. Confirmed `astraweave-audio` is absent from `miri.yml`, `kani.yml`, `mutation-testing.yml`, `coverage.yml` workflows (CI presence only in `ci.yml`). Recovered partial rationale for 5th bus from commit `745c100a8` ("audio ambient" alongside biome materials). Corrected Appendix A `#[non_exhaustive]` claim (irrelevant within same crate). |
| **Status** | Active |
| **Owner notes** | Pure facade crate. No ECS plugin, no system stage registration. Forward-designed editor UI exposes features (HRTF, Doppler, distance model, reverb) that the underlying engine does not yet implement. |

---

## 1. Executive Summary

**What this system does:**
Provides a runtime audio facade over `rodio` 0.17.3 — a multi-bus mixer (master / music / ambient / voice / SFX), 3D spatial panning via per-emitter `SpatialSink`s, A/B-crossfading music and ambient channels, voice-ducked music, and a dialogue-line playback runtime with 4-tier voice-asset resolution (override → voice bank file → folder scan → TTS → beep fallback).

**Why it exists:**
Game audio with low-effort spatial positioning, dialogue voicing, music crossfades, and a TTS adapter point — without forcing consumers to learn `rodio`'s `Sink`/`SpatialSink`/`OutputStream` lifetime story.

**Where it primarily lives:**
- `astraweave-audio/src/engine.rs` — `AudioEngine` (the entire mixer + spatial state, lines 134–431)
- `astraweave-audio/src/dialogue_runtime.rs` — `DialoguePlayer` and asset-resolution chain
- `astraweave-audio/src/voice.rs` — `VoiceBank`, `VoiceSpec`, `TtsAdapter` trait, `SimpleSineTts` (mock_tts feature only)
- `astraweave-audio/src/lib.rs` — flat re-export surface (lines 27–31)
- `tools/aw_editor/src/audio_bridge.rs` — `EditorAudioBridge` (panel-action consumer)
- `tools/aw_editor/src/subsystems/audio_animation.rs::tick_audio_subsystem` — per-frame integration

**Status note:**
The crate's docstring at `lib.rs:8` calls it a "4-bus mixer (master, music, SFX, voice)", but the actual `AudioEngine` struct (`engine.rs:134–163`) holds **five** named bus volumes including a separate `ambient_base_volume` and ambient `MusicChannel`. The docstring is stale relative to the code; use the code as truth.

Two large external doc files describe APIs that **do not exist** in this crate: `docs/src/core-systems/audio.md` references `AudioConfig`, `AudioBackend`, `AudioListener`, `SpatialSound`, `AttenuationModel`, `ReverbZone`, `AudioOcclusion`, etc. (none of these types exist in `lib.rs`'s re-export list). Treat that doc as aspirational, not as a source of API truth.

---

## 2. Authoritative Pipeline

```text
[Asset on disk: .wav / .ogg / .mp3 / .flac]
    │
    │ std::fs::File::open + rodio::Decoder::new (BufReader)
    ▼
[Decoded Source<Item=f32 or i16>]                              ── all dispatch happens in AudioEngine ──
    │
    ├─► play_music(MusicTrack, crossfade_sec)
    │     │ MusicChannel::play (engine.rs:58-94)
    │     │   - picks inactive A/B sink, starts new source at 0 vol
    │     │   - sets crossfade_left = crossfade_sec (≥0.01 clamp)
    │     │   - flips using_a
    │     ▼
    │   [MusicChannel { a: Sink, b: Sink, using_a, crossfade_*, target_vol }]
    │
    ├─► play_ambient(MusicTrack, crossfade_sec)
    │     │ same MusicChannel pattern, separate bus
    │     ▼
    │   [Ambient MusicChannel]
    │
    ├─► play_voice_file(path, approximate_sec) / play_voice_beep(text_len)
    │     │ self.music.duck(self.duck_factor); duck_timer = dur
    │     │ self.voice.append(src); self.voice.play()
    │     ▼
    │   [Voice Sink — non-spatial, scales by voice_base_volume * master]
    │
    ├─► play_sfx_file(path) / play_sfx_beep(hz, sec, gain)
    │     │ self.sfx_bus.append(src); self.sfx_bus.play()
    │     ▼
    │   [SFX Sink — non-spatial bus]
    │
    └─► play_sfx_3d_file(emitter, path, pos) / play_sfx_3d_beep(emitter, pos, hz, sec, gain)
          │ ensure_spatial_sink(emitter) → HashMap<EmitterId, SpatialSink>
          │ sink.set_emitter_position(pos.to_array()); sink.append(src); sink.play()
          ▼
        [Per-emitter SpatialSink — rodio handles distance attenuation + ear-position panning]

──────────────────────────────────────────────────────────────────────
Per-frame: AudioEngine::tick(dt)  (engine.rs:287-299)
    │
    ├─ self.music.update(dt)    ── interpolates A/B volumes across crossfade
    ├─ self.ambient.update(dt)
    └─ if duck_timer > 0: duck_timer -= dt
       on transition to ≤0: music.set_volume(music_base_volume * master_volume)
                            (restores music after voice line completes)

Per-frame: update_listener(ListenerPose { position, forward, up })  (engine.rs:267-274)
    │
    ├─ compute_ears(): right = forward × up (normalized)
    │                  left  = position − right * (ear_sep * 0.5)
    │                  right = position + right * (ear_sep * 0.5)
    └─ for each SpatialSink in self.spat:
         sink.set_left_ear_position(le); sink.set_right_ear_position(re)

──────────────────────────────────────────────────────────────────────
Dialogue path: DialoguePlayer::speak_current(dlg, st)  (dialogue_runtime.rs:33-106)
    │
    ▼
[Node line: { speaker, text }]
    │
    ├─ subtitle_out callback fires first (speaker, text)
    │
    ├─ Tier 1: explicit override?  overrides[dlg.id][node.id] → Path::exists?
    │           → AudioEngine::play_voice_file(fname, None)
    │
    ├─ Tier 2: VoiceBank[speaker].files non-empty?  random choose → Path::exists?
    │           → play_voice_file
    │
    ├─ Tier 3: VoiceBank[speaker].folder scan for *.ogg / *.wav → random choose
    │           → play_voice_file
    │
    ├─ Tier 4: VoiceBank[speaker].tts_voice + TtsAdapter?
    │           → tts.synth_to_path(voice_id, txt, "{folder}/tts_tmp_{u64}.wav")
    │           → play_voice_file(out_path)
    │
    └─ Tier 5 (fallback): AudioEngine::play_voice_beep(txt.len())
```

### Stage-by-stage detail

#### Stage 1: AudioEngine construction (`engine.rs:165-200`)
**Role:** Open the OS default audio device and pre-allocate one `Sink` per non-spatial bus plus two `Sink`s per `MusicChannel` (A/B). The `OutputStream` is held in `_stream` (underscore prefix) for the lifetime of the engine — dropping it would close the device.
**Inputs:** None (uses rodio's `OutputStream::try_default()`).
**Outputs:** A live `AudioEngine` with default master=1.0, music_base=0.8, ambient_base=0.5, voice_base=1.0, sfx_base=1.0, ear_sep=0.2m, pan_mode=StereoAngle, duck_factor=0.4.
**Notes:** Failure here is non-fatal in consumers — `EditorAudioBridge::new` catches the `Err` and runs with `engine: None` (`audio_bridge.rs:67-74`).

#### Stage 2: Master-volume propagation (`engine.rs:202-213`)
**Role:** rodio has no native master volume; this method approximates one by scaling all four bus base-volumes plus every active `SpatialSink`'s volume.
**Inputs:** `v: f32` (clamped to `[0.0, 1.0]`).
**Outputs:** All sink volumes set to `base * master`.
**Notes:** Music/ambient `set_volume` calls go through their `MusicChannel`, which updates `target_vol` (used by the crossfade interpolator on next `tick`). SpatialSinks get the master scalar directly (their per-emitter falloff is computed by rodio from listener+emitter positions).

#### Stage 3: MusicChannel crossfade (`engine.rs:42-132`)
**Role:** Implements per-bus seamless track switching via two `Sink`s that fade in/out over `crossfade_sec`.
**Inputs:** A new `MusicTrack { path, looped }` + crossfade duration.
**Outputs:** The inactive sink begins playing the new source; `crossfade_left` is set; on subsequent `update(dt)` calls, the volumes linearly cross.
**Notes:** When `looped == true`, the source is wrapped in `repeat_infinite()` (`engine.rs:67-70`). A negative or zero crossfade is clamped to `0.01s` (line 91). `duck()` reduces only the currently-active sink's volume, not both (lines 124-131).

#### Stage 4: Voice ducking (`engine.rs:343-371`)
**Role:** Every voice playback call invokes `self.music.duck(self.duck_factor)` and arms `duck_timer`. The timer counts down in `tick`; when it crosses ≤0, music volume is restored.
**Inputs:** `duck_factor` default 0.4 (40% during voice); duck duration = explicit `approximate_sec`, or decoder-reported `total_duration` (clamped 0.1–30s), or fallback heuristic 2.5s. For beeps: `duck_timer = dur + 0.2`.
**Outputs:** `music.target_vol` reduced for the voice window, then restored to `music_base_volume * master_volume`.
**Notes:** Ambient is **not** ducked — only music. The duck only applies to one of the two A/B sinks (whichever is currently `using_a`), per the implementation at lines 124-131.

#### Stage 5: Spatial emitter lifecycle (`engine.rs:389-430, 261-265`)
**Role:** Lazily allocates one `SpatialSink` per `EmitterId: u64` on first 3D play, computes ear positions from listener `forward × up`, and lets rodio handle the actual panning + distance attenuation.
**Inputs:** `EmitterId`, `pos: Vec3`, sound source.
**Outputs:** Inserted/updated entry in `HashMap<EmitterId, SpatialSink>`.
**Notes:** Sinks are **not** auto-evicted when the sound finishes. They persist until `remove_emitter(id)` is called. `active_emitter_count()` returns the HashMap's `len()`, which counts all-ever-played emitters until explicit removal.

#### Stage 6: Dialogue voice resolution (`dialogue_runtime.rs:33-106`)
**Role:** Maps a dialogue node's speaker+text to a playable voice asset (or beep) via a strict 5-tier fallback chain (see pipeline diagram). Always emits the `subtitle_out` callback first (line 41-43).
**Inputs:** `Dialogue`, `DialogueState`, optional `DialogueAudioMap` (per-node overrides), required `VoiceBank` (speaker → folder/files/tts_voice), optional `TtsAdapter`.
**Outputs:** Returns `true` iff some audio path fired (file or beep); `false` only when the node has no `line` field at all.
**Notes:** Tier 4 (TTS) writes to `"{folder}/tts_tmp_{u64::random}.wav"` — these files accumulate; there is no cleanup pass. The TTS adapter is a single-method trait (`synth_to_path`) with no streaming; only the bundled `SimpleSineTts` (mock_tts feature) is shipped, and it generates an enveloped sine wave with `~12 chars/sec` duration scaling.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **AudioEngine** | The complete runtime: holds rodio `OutputStream`, all bus sinks, the spatial-emitter HashMap, listener pose, and ducking state. One per process. | `engine.rs:134`; constructed by every consumer (editor, npc, demos) |
| **EmitterId** | `u64` stable handle for one spatial sound source. Caller-chosen; no allocator. | `engine.rs:9`; `play_sfx_3d_*` calls |
| **ListenerPose** | `{ position: Vec3, forward: Vec3, up: Vec3 }` — the camera/player ears in world space. Update each frame. | `engine.rs:11-16` |
| **PanMode** | `StereoAngle` (angle-based pan + distance attenuation) or `None` (unity pan). Set via `set_pan_mode`. | `engine.rs:19-26` — **set, but never read** by any code in this crate. See §6. |
| **MusicTrack** | `{ path: String, looped: bool }` — input to `play_music` / `play_ambient`. | `engine.rs:28-31` |
| **MusicChannel** | Internal A/B pair of `rodio::Sink`s with crossfade state. Used by both music and ambient buses. | `engine.rs:33-132` (private) |
| **Bus** | Informal name for one mixer channel. The engine has five base volumes: `master_volume`, `music_base_volume`, `ambient_base_volume`, `voice_base_volume`, `sfx_base_volume`. | `engine.rs:148-153` |
| **VoiceBank** | `HashMap<speaker_name, VoiceSpec>` loaded from TOML. Maps a dialogue speaker to a folder of voice clips and an optional TTS voice id. | `voice.rs:23-25`; `load_voice_bank` |
| **VoiceSpec** | `{ folder, files, tts_voice }` — the resolution recipe for one speaker. Empty `files` triggers folder scan for `.ogg`/`.wav`. | `voice.rs:11-20` |
| **DialogueAudioMap** | Per-node audio overrides: `HashMap<dialogue_id, HashMap<node_id, filename>>`. TOML-loaded. Highest-priority tier in the voice resolution chain. | `dialogue_runtime.rs:11-14` |
| **TtsAdapter** | `Send + Sync` trait with single method `synth_to_path(voice_id, text, out_path)`. Pluggable; only `SimpleSineTts` ships. | `voice.rs:36-39` |
| **Duck factor** | Multiplier applied to music volume while a voice is speaking. Default 0.4. Setting to 1.0 disables ducking. | `engine.rs:198, 251-253` |
| **Ear separation** | Distance (meters) between left/right ear positions used by spatial panning. Default 0.2m; clamped to [0.05, 1.0]. | `engine.rs:241-248`, `276-285` |

### Terms to NOT confuse

- **Music bus vs Ambient bus:** Both are `MusicChannel` instances (A/B crossfade pair), but they are independent — music and ambient can layer. Voice ducking only affects music, not ambient (`engine.rs:347, 367, 291-298`).
- **Spatial Sink vs SFX Sink:** `sfx_bus` (`engine.rs:143`) is a single non-spatial `Sink` for one-shot 2D sounds. `spat: HashMap<EmitterId, SpatialSink>` (`engine.rs:146`) is the per-emitter spatial pool. `play_sfx_file` uses the former; `play_sfx_3d_file` uses the latter. They are **not** wired together — a "stop all SFX" call would need to walk both.
- **PanMode (this crate) vs SpatialPreset (editor panel):** The editor panel's `SpatialPreset` (Standard/Headphones/Speakers/Surround/VR) influences `ear_separation` (panel: `audio_panel.rs:163-171`), but the `AudioAction::SetSpatialPreset` variant is currently a comment-only no-op in `EditorAudioBridge::process_actions` (`audio_bridge.rs:165-167`). `PanMode` (this crate's only spatial-mode enum) has only two variants and is not exposed in the panel.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Filesystem (assets) | `std::fs::File::open` + `rodio::Decoder::new` inside `play_music` / `play_voice_file` / `play_sfx_file` / `play_sfx_3d_file` | `.wav` / `.ogg` / `.mp3` / `.flac` byte streams | All errors `?`-propagated wrapped with `anyhow!("open … {}: {}", path, e)`. Voice-file `Result` is *not* discarded (`engine.rs:344`). |
| TOML config | `load_voice_bank(path)`, `load_dialogue_audio_map(path)` (`voice.rs:27-31`, `dialogue_runtime.rs:16-20`) | `VoiceBank` and `DialogueAudioMap` structs | Failures return `Err` for caller to handle. |
| Camera / player transform | `AudioEngine::update_listener(ListenerPose)` | Per-frame position+forward+up | Editor: `audio_bridge.rs:211-219` calls this from `tick_audio_subsystem` using viewport camera (`audio_animation.rs:27-31`). Demos: same pattern inline. |
| Gameplay dialogue state | `astraweave-gameplay::dialogue::Dialogue` + `DialogueState` (re-exported through `astraweave-gameplay`) | Conversation graph | The `astraweave-gameplay` dep is the **only** workspace dependency of `astraweave-audio` (`Cargo.toml:16`). |
| Editor UI panel | `AudioAction` enum drained from `dock_tab_viewer.take_audio_actions()` | Slider drags, mute toggles, track-play clicks, emitter add/remove | Editor: `tools/aw_editor/src/subsystems/audio_animation.rs:14-17`; bridge processes via `audio_bridge.rs:111-208`. |
| NPC commands | `EngineCommandSink::say()`, `call_guards()` (`astraweave-npc/src/runtime.rs:37-50`) | Text → beep, position → 3D beep | `play_sfx_3d_beep` `Result` is discarded with `let _ =` at `runtime.rs:49`. Voice path uses `play_voice_beep` only (TTS not wired in). |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| rodio 0.17.3 (vendored audio device driver) | `Sink::append`, `Sink::set_volume`, `Sink::play`, `SpatialSink::set_emitter_position`, `SpatialSink::set_left_ear_position`, `SpatialSink::set_right_ear_position` | PCM frames + control commands | rodio internally drives the OS audio device (WASAPI on Windows per `cpal` selection). No code in `astraweave-audio` reaches below rodio. |
| Editor `AudioPanel` (display) | `AudioEngine::active_emitter_count()`, `EditorAudioBridge::stats()` → `AudioStats { active_voices, active_music_channels, active_emitters, cpu_usage_percent, latency_ms, … }` | UI stats display | `audio_bridge.rs:221-238`. Note: `active_voices`, `active_music_channels`, `memory_usage_mb`, `buffer_underruns`, `sample_rate` are hardcoded constants in the editor bridge — not measured from the engine. |

### Bidirectional / Coupled

- **AudioEngine ↔ EditorAudioBridge:** The bridge owns one `Option<AudioEngine>` (`audio_bridge.rs:45`). Panel actions flow in; engine state (active emitter count, last tick time) flows out as stats. The bridge is fully consumed by `tick_audio_subsystem` once per editor frame.

### Crates that declare but do not import the dependency

- **`examples/veilweaver_demo/Cargo.toml:27`** declares `astraweave-audio = { path = "../../astraweave-audio" }` but `rg 'astraweave_audio|AudioEngine' examples/veilweaver_demo/src` returns no matches as of `a2474c5b7`. This appears to be unused dependency residue.

### Documentation references with no code backing (aspirational docs)

- **`docs/src/core-systems/audio.md`** uses types like `AudioConfig`, `AudioBackend`, `BackendType::WASAPI`, `AudioListener`, `SpatialSound`, `AttenuationModel`, `Obstruction`, `MusicManager`, `MusicLayer`, `TransitionType`, `MusicState`, `SfxManager`, `SoundPool`, `SoundPriority`, `SoundCategory`, `Reverb`, `LowPassFilter`, `HighPassFilter`, `AudioProcessor`, `AudioBuffer`, `AudioBudget`. **None** of these appear in `astraweave-audio/src/lib.rs`'s re-exports (lines 27-31). The doc is either describing a planned/aspirational API or referring to a different/forked crate.
- **`docs/src/api/audio.md`** similarly references `SpatialAudio`, `AudioMixer`, `Bus`, `SpatialSource`, `MusicPlayer`, `ReverbZone`, `ReverbPreset`, `AudioOcclusion` — also absent from the actual crate.
- **`docs/src/reference/crates.md:158`** suggests `use astraweave_audio::prelude::*;` but no `prelude` module exists in `lib.rs`.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-audio/src/lib.rs` | Crate facade: 4-line `pub mod` declarations + 5-line `pub use` re-exports | Active | `#![forbid(unsafe_code)]` (line 1). 31 lines total. Docstring says "4-bus mixer" but the engine has 5 buses — stale. |
| `astraweave-audio/src/engine.rs` | `AudioEngine`, `MusicChannel` (private), `ListenerPose`, `PanMode`, `EmitterId`, `MusicTrack`. 1885 lines (most of which are tests inline). | Active | Single file = entire mixer. Lines 1-431 are production code; lines 484-1885 are `#[cfg(test)]`. |
| `astraweave-audio/src/dialogue_runtime.rs` | `DialoguePlayer`, `DialogueAudioMap`, `load_dialogue_audio_map`. The 5-tier voice resolution chain lives here. | Active | 834 lines; ~110 production + extensive tests. |
| `astraweave-audio/src/voice.rs` | `VoiceBank`, `VoiceSpec`, `TtsAdapter` trait, `SimpleSineTts` (mock_tts only), `load_voice_bank`. | Active | 241 lines. `SimpleSineTts` and its `hound` dep are feature-gated by `mock_tts` (`Cargo.toml:18-23`). |
| `astraweave-audio/src/mutation_tests.rs` | Test-only module for cargo-mutants resistance. | Active (tests) | 2008 lines, all `#[cfg(test)]`. Targets boundary conditions, operator substitutions, return-value mutations. |
| `astraweave-audio/tests/*.rs` (14 files) | Integration test suite: stress, edge cases, behavioral correctness, mutation resistance, dialogue files, error handling, resource cleanup, integration. | Active (tests) | Total integration test LoC: ~7,063. Several use `#[serial]` (via `serial_test` 3.x) because they share the global default audio device. |
| `astraweave-audio/tests/fixtures/` | Real audio fixtures (`.wav`, `.ogg`) plus generator scripts (`generate_audio.rs`, `generate_audio_files.ps1`, `generate_fixtures.rs`). | Active | Some integration tests are `#[ignore]`-gated until fixtures present (`integration_tests.rs:30`). |
| `astraweave-audio/tests/assets/` | Pre-generated `.wav` test sounds (beeps + voices + music). | Active | Generated by `tests/test_asset_generator.rs`. |
| `astraweave-audio/benches/audio_benchmarks.rs` | Criterion benches for engine init, listener update, master-volume changes, spatial sink play. Includes "MISSION-CRITICAL CORRECTNESS ASSERTIONS" header (lines 5-15) — verifies `is_finite()` and value ranges inside the bench loop. | Active | 323 lines. Asserts fail the bench if invariants break. |
| `tools/aw_editor/src/audio_bridge.rs` | `EditorAudioBridge` — non-fatal `AudioEngine` owner, `AudioAction` → engine command dispatcher, asset-folder scanner. | Active | 274 lines. **10 of ~25 `AudioAction` variants are explicit no-ops with "// noted for future DSP integration" comments** (lines 159-205). See §6. |
| `tools/aw_editor/src/panels/audio_panel.rs` | `AudioPanel` — egui-driven UI for buses, music, spatial, emitters, preview. Emits `AudioAction` enum values via `take_audio_actions()`. | Active | 2086 lines. UI is far richer than the engine's actual capabilities (see §6). |
| `tools/aw_editor/src/subsystems/audio_animation.rs::tick_audio_subsystem` | Wiring: drain actions → bridge → tick → stats → listener-update each editor frame. | Active | The only path from editor camera position into `AudioEngine`. |
| `astraweave-npc/src/runtime.rs` (`EngineCommandSink`) | NPC `say()` → `play_voice_beep`; `call_guards()` → `play_sfx_3d_beep`. Holds `&mut AudioEngine`. | Active | The only non-editor, non-example production consumer in the workspace. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care.
- **Active (tests)**: Carries no runtime weight but exercises invariants.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| Crate-level docstring "4-bus mixer (master, music, SFX, voice)" | `lib.rs:8` | Stale | Engine has 5 buses; ambient is separate. Doc string drifted from code at some point. |
| `docs/src/core-systems/audio.md` aspirational API surface (`AudioConfig`, `AudioListener`, `SpatialSound`, `AttenuationModel`, `ReverbZone`, `MusicManager`, `MusicLayer`, `SfxManager`, `SoundPool`, `AudioMixer`, `Bus`, …) | `docs/src/core-systems/audio.md`, `docs/src/api/audio.md`, `docs/src/reference/crates.md`, `docs/src/api/index.md` | Reference-only, code-absent | None of these types exist in the actual crate. Origin: `git log --diff-filter=A` traces both core-systems and api audio docs to commit `28bc94f21` (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") authored by `Copilot <198982749+Copilot@users.noreply.github.com>`. The commit added ~80 doc files in one sweep with no corresponding code changes. This identifies the docs as **AI-generated structural placeholders**, not historical drift or copy-paste from another engine. Disposition (delete / rewrite / keep as roadmap) remains Andrew's call — see §11. |
| Editor `AudioPanel` UI surface vs `AudioEngine` capability surface | `tools/aw_editor/src/panels/audio_panel.rs` + `tools/aw_editor/src/audio_bridge.rs:165-182` | Forward-designed | Panel has UI for HRTF toggle, Doppler toggle, distance model selection, reverb environment, spatial preset, crossfade duration storage, shuffle/loop playlist behavior. Bridge's match arms for these `AudioAction` variants are bodyless or comment-only. Engine has no DSP for them. |
| Asset-discovery file extensions: editor scans `wav/ogg/mp3/flac`; dialogue folder-scan checks only `ogg/wav` | `audio_bridge.rs:30-34` vs `dialogue_runtime.rs:74-77` | Active divergence | Not necessarily a bug — dialogue is intentionally a smaller subset — but a future asset-format addition needs both touchpoints updated. |
| `examples/veilweaver_demo` Cargo.toml dep on `astraweave-audio` | `examples/veilweaver_demo/Cargo.toml:27` | Unused residue | No source file in the demo imports the crate as of `a2474c5b7`. |

### Naming collisions

- **"PanMode" (this crate) vs "SpatialPreset" (editor panel) vs "DistanceModel" (editor panel):** All three name "how spatial audio behaves" but operate at different layers and with different vocabularies. `PanMode` has 2 variants (`StereoAngle`, `None`) and is `set_pan_mode`-only — **never read** by any code in the crate (the field exists but is not branched on in any audio decision). `SpatialPreset` has 5 variants and is mapped to ear-separation values inside the panel (`audio_panel.rs:163-171`) but its `AudioAction::SetSpatialPreset` is a no-op in the bridge. `DistanceModel` is enumerated in the panel but its action is a no-op as well. Future direction: [Reasoning not recovered from available sources].
- **"Bus" (gh-pages/audio.md) vs no `Bus` type in code:** `gh-pages/audio.md:8` claims "5-bus mixer". There is no `Bus` enum, struct, or trait in the crate. The buses are just five `f32` fields on `AudioEngine` plus four `Sink`/`MusicChannel` instances. The conceptual "bus" never gets a type.

### Known cognitive traps

- **Trap:** `AudioEngine::active_emitter_count()` does not equal "currently audible emitters" or "playing emitters".
  - **Why it's confusing:** The name suggests "in flight". The code (`engine.rs:256-258`) returns `self.spat.len()`, which is the HashMap size. Sinks are never auto-removed when their queue drains.
  - **What's actually true:** It's "emitter slots that have ever been allocated and not explicitly removed via `remove_emitter`". Long-running sessions will see this number monotonically grow.

- **Trap:** `set_pan_mode(PanMode::None)` does not actually disable spatialization on existing `SpatialSink`s.
  - **Why it's confusing:** The setter exists and accepts an enum.
  - **What's actually true:** The `pan_mode` field is stored (`engine.rs:158, 215-217`) but is **never read** by any other method in the crate. Spatial sinks already created continue using rodio's spatial panning regardless. The `test_pan_mode_*` mutation tests assert the setter+getter contract but no behavioral test verifies pan mode actually changes output.

- **Trap:** Voice ducking duck-restores `music` only; `ambient` is not ducked.
  - **Why it's confusing:** The naming "voice ducking" suggests "everything under voice is reduced". Ambient is also background audio.
  - **What's actually true:** `play_voice_*` calls `self.music.duck(...)` (`engine.rs:347, 367`). Ambient is updated independently each tick and is unaffected by `duck_timer`. If you want ambient to duck too, that's a new code path.

- **Trap:** The `_stream: OutputStream` field name with the underscore prefix.
  - **Why it's confusing:** Looks like an unused field.
  - **What's actually true:** rodio's `OutputStream` must outlive every `Sink` created from its handle; dropping it closes the device. The `_` only silences a "field never read" warning while the value's drop order matters.

- **Trap:** Crossfade duration of `0.0` (or negative) is silently treated as `0.01s`, not "no crossfade".
  - **Why it's confusing:** A `play_music(track, 0.0)` call looks like "switch instantly".
  - **What's actually true:** `engine.rs:91`: `self.crossfade_time = crossfade.max(0.01)`. A real instant-switch needs the prior track to be `stop_music()`'d first.

- **Trap:** Dialogue TTS leaves temp files behind.
  - **Why it's confusing:** TTS feels ephemeral.
  - **What's actually true:** `dialogue_runtime.rs:96` writes `"{folder}/tts_tmp_{random_u64}.wav"` and never deletes it. Multiple dialogue passes accumulate `tts_tmp_*.wav` files in the voice folder.

- **Trap:** Two consumers picking the same `EmitterId` silently merge their spatial sounds and overwrite the emitter position.
  - **Why it's confusing:** `EmitterId` is `u64` with no allocator and no sentinel; callers freely pick values. The name suggests "unique handle".
  - **What's actually true:** `ensure_spatial_sink` (`engine.rs:421-430`) only creates a new `SpatialSink` if the key is absent; otherwise the existing sink is reused. `play_sfx_3d_file` / `play_sfx_3d_beep` (`engine.rs:389-419`) then call `s.set_emitter_position(pos.to_array())` followed by `s.append(src)` on the reused sink, overwriting the position of any still-queued earlier sound. A test demonstrates this exact behavior: `test_emitter_position_update` (`engine.rs:1042-1058`) reuses ID `42` at two positions and asserts the HashMap still contains only one entry. There is no name collision check or warning. Game code that uses derived IDs (e.g. entity-based) must coordinate to avoid accidental sharing.

---

## 7. Decision Log

### Decision: rodio as the runtime audio backend
- **Date:** Pre-`a2474c5b7` (no recovered commit boundary)
- **Status:** Accepted
- **Context:** `Cargo.toml:195` pins `rodio = "0.17"` at the workspace level. Direct dep of `astraweave-audio`. No alternative audio crate (kira, cpal-direct, oddio, etc.) appears in `Cargo.lock`.
- **Decision:** Use rodio 0.17.3 as sole audio runtime.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - `rodio::OutputStream` (held by `AudioEngine._stream`) wraps a `cpal::Stream`, which carries an explicit `NotSendSyncAcrossAllPlatforms(PhantomData<*mut ()>)` marker (`cpal-0.15.3/src/platform/mod.rs:73, 736`). This makes `OutputStream` — and therefore `AudioEngine` — `!Send + !Sync` (verified by source inspection of the vendored rodio 0.17.3 and cpal 0.15.3 in `~/.cargo/registry/`). cpal's own comment (`platform/mod.rs:67-72`) cites Android AAudio thread-safety as the underlying constraint. The `Sink` and `SpatialSink` types themselves contain only `Arc`/`Mutex`/`AtomicBool` fields (rodio `sink.rs:18-26`, `spatial_sink.rs:11-14`) and so are *individually* `Send + Sync`; it is the `OutputStream` field, not the sinks, that blocks `AudioEngine` from being placed in cross-thread containers.
  - No code in this crate makes `AudioEngine` an ECS `Resource` (which would require `Send + Sync` per `astraweave-ecs`'s `Resource` trait bound; see also §11).
  - rodio's spatial panning uses ear positions, hence `compute_ears()` and the `ear_sep` parameter, rather than a HRTF impulse response approach.
  - rodio has no global "master volume" concept — hence the per-call `set_master_volume` cascade (`engine.rs:202-213`).

### Decision: 5 buses (master / music / ambient / voice / SFX)
- **Date:** Initial 4-bus version landed in commit `a793b32cc` (2025-09-04, "Add audio engine with music and SFX functionality"). The 5th (ambient) bus was added in the broader sweeping commit `745c100a8` ("Mutation-resistant test suites … Asset additions (HDRI catalogs, biome materials, audio ambient) … biome transition, weather system, scene environment …"), confirmed via `git log -S "ambient_base_volume"`.
- **Status:** Accepted (live in `engine.rs:140-153`)
- **Context:** The crate docstring (`lib.rs:8`) still says "4-bus" — it predates the ambient addition. `gh-pages/audio.md:8` says "5-bus mixer (Master/Music/SFX/Ambient/Voice)" — the public-facing doc was updated; the in-crate docstring was not. The ambient bus arrived alongside biome material work, suggesting it was added to layer biome ambience under music. The `engine.rs:141` field comment confirms: `ambient: MusicChannel, // biome ambient loops (same A/B crossfade pattern)`.
- **Decision:** Five concurrent channels: master (scalar over the others), music (crossfade A/B), ambient (crossfade A/B), voice (single `Sink`), sfx (single `Sink`), plus per-emitter spatial `SpatialSink`s.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:** Music and ambient can layer (e.g., area BGM + biome ambience). Voice ducks music but not ambient. SFX is a single shared sink — overlapping SFX queue serially through one `Sink` rather than mixing in parallel (per rodio's `Sink::append` semantics).

### Decision: AudioEngine is not an ECS Resource
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted by construction
- **Context:** `astraweave-audio/Cargo.toml` has zero `astraweave-ecs` dependency. No `App::add_resource(AudioEngine)` or `world.get_resource_mut::<AudioEngine>` pattern exists anywhere in the workspace.
- **Decision:** Consumers hold `AudioEngine` directly (`&mut AudioEngine` parameter or owning struct field).
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - There is no PRESENTATION-stage audio system. Audio is ticked by whatever owns the engine: in the editor, `tick_audio_subsystem`; in demos, inline in the winit event loop's redraw handler.
  - Multiple consumers in the same process must arrange single-ownership; there's no shared `Arc<RwLock<AudioEngine>>` pattern in the codebase.
  - This sidesteps any `Send + Sync` question rodio types would otherwise raise. See §11.

### Decision: TtsAdapter is a single-method synth-to-file trait
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted
- **Context:** `voice.rs:33-39`. The trait is `Send + Sync` and has one method `synth_to_path(voice_id, text, out_path)` returning `Result<()>`.
- **Decision:** TTS produces a finished WAV/OGG file on disk; the audio engine then plays it like any other voice file. No streaming, no chunked synthesis, no in-process buffer handoff.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - TTS latency = (synth time) + (file write) + (file read by decoder). Per-line, not per-token.
  - Temp WAVs accumulate (see §6 traps).
  - The trait can be implemented by any backend (local ONNX, cloud API, espeak, etc.) because it's a file-out boundary.

### Decision: Dialogue resolution = 5-tier fallback chain
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (`dialogue_runtime.rs:33-106`)
- **Context:** Each speaker may have explicit per-node overrides, a named voice folder, an arbitrary set of `.ogg`/`.wav` clips, an optional TTS voice id, or none of the above.
- **Decision:** Try in order — explicit override file → speaker `files` list (random pick) → speaker folder scan (random pick) → TTS synthesis → beep proportional to text length.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Dialogue *always* produces audio output (worst case: beep). Subtitles always fire first regardless.
  - The 5-tier chain hard-codes file-system priority. Adding a network-fetched voice source would require modifying `speak_current` directly.

### Decision: `#![forbid(unsafe_code)]` at crate root
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (`lib.rs:1`)
- **Context:** Audio code is a high-leverage real-time path; mistakes are audible.
- **Decision:** Zero unsafe in the audio crate. All `unsafe` operations are buried inside rodio or its dependencies.
- **Alternatives considered:** None reasonable for this layer.
- **Consequences:** Cannot drop down to FFI for low-latency callbacks without modifying this attribute. Not needed in practice — rodio handles the device driver layer.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `master_volume ∈ [0.0, 1.0]` | Yes | `set_master_volume(v) = v.clamp(0.0, 1.0)` (`engine.rs:203`). Mutation tests cover the boundary (`mutation_tests.rs:23-50`). |
| 2 | All bus base volumes ∈ [0.0, 1.0] (music, ambient, voice, sfx) | Yes | Per-setter `.clamp(0.0, 1.0)` (`engine.rs:221, 228, 235, 333`). |
| 3 | `ear_sep ∈ [0.05, 1.0]` | Yes | `set_ear_separation` clamps to [0.05, 1.0] (`engine.rs:242`). |
| 4 | `duck_factor ∈ [0.0, 1.0]` | Yes | `set_duck_factor` clamps to [0.0, 1.0] (`engine.rs:252`). |
| 5 | `crossfade_time ≥ 0.01` (no instant crossfade) | Yes | `MusicChannel::play` clamps with `.max(0.01)` (`engine.rs:91`). |
| 6 | Voice playback always sets `duck_timer > 0` | Yes | `engine.rs:349-356` (file path) and `engine.rs:368` (beep) both set `duck_timer`. Mutation test `mutation_play_voice_beep_duck_timer_formula` asserts exact value (`engine.rs:1859-1869`). |
| 7 | `tick(dt)` decreases `duck_timer` by exactly `dt` while positive | Yes | `engine.rs:292`. Mutation test `mutation_tick_duck_timer_subtracts_dt` asserts (`engine.rs:1872-1884`). |
| 8 | Music volume is restored to `music_base_volume * master_volume` when duck timer crosses ≤ 0 | Yes | `engine.rs:293-298`. Mutation test `mutation_tick_duck_restores_music_target_vol` asserts (`engine.rs:1759-1784`). |
| 9 | After construction, both `MusicChannel`s have `using_a == true` and `crossfade_left == 0.0` | Yes | `engine.rs:51, 53-54`. Mutation tests assert (`engine.rs:1598-1612, 1648-1665`). |
| 10 | `compute_ears` produces ears at distance exactly `ear_sep` for non-degenerate forward/up | Yes (with floating tolerance) | `engine.rs:276-285`. Mutation test asserts within `0.01` of `0.2m` (`engine.rs:1821-1836`). |
| 11 | Dialogue `speak_current` returns `Ok(false)` iff the node has no `line` | Yes | `dialogue_runtime.rs:34-37`. Tested by `test_dialogue_player_no_line` (`dialogue_runtime.rs:178-206`). |
| 12 | Dialogue `speak_current` emits the subtitle callback before any audio resolution attempt | Yes | `dialogue_runtime.rs:41-43`. Tested by `test_dialogue_player_subtitle_callback`. |
| 13 | All voice-file/sfx-file/3d-sfx-file `Result`s are propagated to caller (not silently discarded) | Yes | `engine.rs:344, 374, 390` all use `?` with `anyhow!()` context. Editor caller logs via `tracing::warn!` (`audio_bridge.rs:151-153, 193-195`). NPC caller uses `let _ =` for `call_guards` 3D beep (`runtime.rs:49`) — this is an exception, intentional. |
| 14 | `AudioEngine::new()` failure is non-fatal in the editor — falls back to `engine: None` and runs without audio | Yes | `audio_bridge.rs:67-75`. |
| 15 | TTS is only available when the `mock_tts` feature is enabled (in this crate's bundled adapter) | Yes (compile-time) | `voice.rs:43, 49, 59` all `#[cfg(feature = "mock_tts")]`. The `TtsAdapter` trait is unconditional; only the bundled implementation is gated. |
| 16 | The `_stream: OutputStream` field outlives all `Sink`s created from `handle` | Doc-only | `engine.rs:136`. rodio invariant; documented by the underscore prefix preventing "unused" warning. If you move `_stream` out, all sinks drop silently. |

---

## 9. Performance & Resource Profile

### Hot paths

- **`AudioEngine::tick(dt)`** — runs every editor frame (`audio_animation.rs:18`) and every demo redraw. Does two `MusicChannel::update` calls (each a few volume math ops) and one duck-timer subtraction. Sub-microsecond budget. Exercised inside the `listener_movement_single_emitter` and `listener_movement_10_emitters` benches (`audio_benchmarks.rs:130-184`), which call `engine.tick(0.016)` once per iteration. There is no dedicated `tick`-only bench.
- **`update_listener(pose)`** — runs every editor frame for camera tracking. Walks `self.spat` once per call to push ear positions. With N emitters, O(N) per call. Default sessions have 0–few emitters; the editor `AudioPanel`'s "Add Emitter" button is the only path that scales N up in the editor.
- **`SpatialSink::set_emitter_position` + per-frame ear repositioning** — happens N times per frame on listener update; rodio internally recomputes the per-sample gain.

### Cold paths

- **Asset decoding** (`Decoder::new(BufReader::new(file))`) — runs once per `play_music` / `play_voice_file` / `play_sfx_file` call. Whole-file read + format parse before the source plays. For multi-MB OGG/MP3 files this can be tens of milliseconds. There is no streaming or async loading — the call blocks the caller until decode begins.
- **Dialogue voice resolution** — `Path::exists` checks plus possibly a `fs::read_dir` traversal for folder-scan tier (`dialogue_runtime.rs:71-92`). Once per spoken line.
- **TTS synthesis** (`SimpleSineTts::synth_to_path`) — writes a WAV file, sample by sample, with envelope shaping. ~150K samples for a 7-second clip at 22050 Hz. Real TTS adapters would have very different costs.

### Resource ownership

- **`OutputStream`** — owned by `AudioEngine._stream`. One per process. Closed on `AudioEngine` drop.
- **`Sink`** (voice, sfx) — owned by `AudioEngine`. Live for the engine's lifetime.
- **`MusicChannel`** (music, ambient) — owns 2 `Sink`s each. `MusicChannel::play` creates fresh `Sink::try_new(handle)` per track switch and drops the previous one explicitly via `.stop()` (`engine.rs:75, 83`).
- **`HashMap<EmitterId, SpatialSink>`** — owned by `AudioEngine`. Sinks live until `remove_emitter` is called. **Memory leak risk:** an editor session that adds many emitters and never removes them grows this HashMap unboundedly. See §6 cognitive trap.
- **TTS temp files** — leaked to disk under `{voice_folder}/tts_tmp_{random_u64}.wav`. No cleanup routine. See §6.

---

## 10. Testing & Validation

- **Unit tests:** Inline `#[cfg(test)] mod tests` blocks inside each source file. ~80 tests in `engine.rs` (lines 484-1885), ~25 in `dialogue_runtime.rs` (lines 109-834), ~10 in `voice.rs` (lines 95-241).
- **Integration tests:** 14 files under `astraweave-audio/tests/` totaling ~7,063 LoC:
  - `audio_engine_tests.rs`, `integration_tests.rs` (workflows; some `#[ignore]`-gated on fixtures)
  - `behavioral_correctness_tests.rs`, `dialogue_and_voice_tests.rs`, `dialogue_file_tests.rs`
  - `edge_case_tests.rs`, `advanced_edge_cases.rs`, `error_handling_tests.rs`
  - `file_based_audio_tests.rs`, `stress_tests.rs`, `resource_cleanup_tests.rs`
  - `additional_integration_tests.rs`, `mutation_resistant_comprehensive_tests.rs`
  - `test_asset_generator.rs`, `generate_fixtures.rs` (asset-prep harness, not test runners)
- **Test isolation:** Several test files use `#[serial]` from `serial_test` 3.x (Cargo.toml:28; observed in `dialogue_file_tests.rs`, `file_based_audio_tests.rs`, `test_asset_generator.rs`, `advanced_edge_cases.rs`) because all tests share the global OS default audio device — parallel tests would contend.
- **Mutation testing:** Dedicated `src/mutation_tests.rs` (2008 lines, all `#[cfg(test)]`) plus `tests/mutation_resistant_comprehensive_tests.rs` (569 lines). Targets cargo-mutants survivor classes — boundary conditions, operator substitutions, return-value mutations on test accessors. See invariants 1–10 above for individual covered properties.
- **Miri validation:** Confirmed not present in `.github/workflows/miri.yml` (workspace grep `grep -l astraweave-audio .github/workflows/miri.yml` returns no match as of `a2474c5b7`). `#![forbid(unsafe_code)]` (`lib.rs:1`) leaves nothing for Miri to validate at the crate boundary; any UB would live in rodio/cpal. Similarly absent from `kani.yml`, `mutation-testing.yml`, `coverage.yml`; present in `ci.yml` only (lines 177, 293, 315).
- **Benchmarks:** `benches/audio_benchmarks.rs` — criterion 0.5 (323 lines). Includes inline `[CORRECTNESS FAILURE]`-prefixed assertions in the bench loops (lines 5-15 header, lines 17-67 validators) so benchmark runs double as smoke tests.
- **Manual validation:** Audible — `examples/audio_spatial_demo`, `examples/dialogue_voice_demo`, `examples/dialogue_audio_cli`, `examples/npc_town_demo` all surface audio interactively.

---

## 11. Open Questions / Parked Decisions

- **Send/Sync story for `AudioEngine`:** Verified `!Send + !Sync` via source inspection of vendored rodio 0.17.3 + cpal 0.15.3. `cpal::Stream` (held transitively by `AudioEngine._stream`) carries an explicit `NotSendSyncAcrossAllPlatforms(PhantomData<*mut ()>)` marker (`cpal-0.15.3/src/platform/mod.rs:73, 736`) — the constraint propagates up through `rodio::OutputStream` (`stream.rs:16-19`) into `AudioEngine`. Consequence: the engine cannot be moved across threads, cannot be wrapped in `Arc<RwLock<_>>` for cross-thread access, and cannot be stored as an ECS `Resource` (which requires `Send + Sync` per `astraweave-ecs::Resource`). The crate consequently bypasses the ECS entirely. A speculative comment in `examples/ui_menu_demo/src/main.rs:373` suggests `Arc<Mutex<AudioEngine>>` "in production code" — this would not compile given the Send/Sync constraint. Is single-threaded audio an intentional design, or a constraint that should be addressed at the rodio/cpal level? Andrew's call.

- **`PanMode` field is set but never read.** Comprehensive workspace grep for `pan_mode|PanMode` (2026-05-12) returns 30+ matches across `src/`, `tests/`, `benches/` — every match is either the type definition, an enum variant, a setter call (`set_pan_mode`), or a getter (`test_pan_mode`). No code reads `self.pan_mode` and branches on it. The dedicated benchmark `pan_mode_switching` (`audio_benchmarks.rs:187-205`) calls `set_pan_mode` in a tight loop but only validates "Mode switch should not crash". Stress tests (`tests/stress_tests.rs:329, 344, 361`), resource-cleanup tests (`tests/resource_cleanup_tests.rs:142-146, 427`), file-based tests (`tests/file_based_audio_tests.rs:476-489`), and mutation tests (`tests/mutation_resistant_comprehensive_tests.rs:15-42, 311-315`) all exercise the setter but none assert audio output differs between modes. The field is stored at `engine.rs:158` and updated at `engine.rs:215-217`; nowhere else. Was this intended for a future implementation, or is it vestigial? Andrew's call on whether to wire it or remove it.

- **Editor `AudioAction` no-op variants:** `SetCrossfadeDuration`, `ToggleShuffle`, `ToggleLoop`, `SetSpatialPreset`, `ToggleHrtf`, `ToggleDoppler`, `SetDistanceModel`, `SetReverbEnvironment`, `ToggleReverb`, `StopPreview` — all have comment-only or empty match arms in `audio_bridge.rs:159-205`. Are these awaiting engine-side implementation (forward-designed UI), or should the panel hide them until the engine catches up? See §6.

- **Disposition of AI-generated aspirational audio docs:** The factual origin is now known (see §6 row, closed via deep investigation on 2026-05-12): `docs/src/core-systems/audio.md` and `docs/src/api/audio.md` were both created in commit `28bc94f21` (2025-09-08) by GitHub Copilot bot as part of a sweep titled "Create comprehensive bespoke wiki with 51-section documentation structure (#34)" — a structural-placeholder generation, not a record of prior implementation. Subsequent `029bccd5b` added performance content but did not change the surface area claims. Remaining decision: should these docs be (a) deleted, (b) rewritten to match the actual `astraweave-audio` API, or (c) retained as a roadmap for a future audio API rewrite? Andrew's call.

- **TTS temp file lifecycle:** `dialogue_runtime.rs:96` writes `tts_tmp_*.wav` under the voice folder with no cleanup. Long-running sessions accumulate. Is a cleanup pass needed (on session end? on each `speak_current` call after playback finishes?) or is asset-folder pollution acceptable for the current scale? Andrew's call.

- **Spatial sink lifecycle:** `HashMap<EmitterId, SpatialSink>` grows without bound; only explicit `remove_emitter(id)` evicts. Verified no auto-prune path exists in `astraweave-audio/src/` (no `retain`/`drain` on `self.spat` in `engine.rs`) and in the editor (`audio_bridge.rs` only calls `remove_emitter` on explicit `AudioAction::RemoveEmitter` from the panel, lines 197-199). Who owns the "garbage collect inactive emitters" responsibility — engine, bridge, or game logic? Andrew's call.

- **`active_emitter_count()` semantics:** Returns HashMap size, which conflates "ever allocated" with "currently playing". The editor's `AudioStats.active_emitters` (`audio_bridge.rs:222-228`) uses this value, so the editor's stats display is similarly imprecise. Is the user-facing value supposed to be "playing right now" (would need different logic) or "active slot count" (current behavior)? Andrew's call.

- **`active_voices` and `active_music_channels` in `AudioStats` are hardcoded** (`audio_bridge.rs:229-230`). `git log -p --follow tools/aw_editor/src/audio_bridge.rs` shows these values (`if self.engine.is_some() { 1 } else { 0 }` for voices, `{ 2 } else { 0 }` for music_channels) were present in the very first commit of `audio_bridge.rs` — `d9aa91d95` (`feat: implement animation and audio bridges, add movement scripts`) — and have not been modified since. No commit message explains the magic numbers; no test asserts them. The most plausible structural reading: `1` matches the engine's single `voice: Sink` field (`engine.rs:142`), and `2` matches the two `MusicChannel`s (music + ambient at `engine.rs:140-141`). The values flow into the editor's Mixer tab via `audio_panel.rs:931-970` (`show_audio_stats`), so the user sees "Active Voices: 1, Active Music Channels: 2" whenever the engine is alive, regardless of actual playback. Is this a temporary placeholder until engine-side counters exist, or intentional structural reporting? Andrew's call.

- **`examples/veilweaver_demo` dependency on `astraweave-audio`** is declared in Cargo.toml but unused in `src/`. `git log --all -S "astraweave_audio" -- examples/veilweaver_demo` (2026-05-12) returns **zero** matching commits — confirming the import was never written, the dependency was added pre-emptively (commit `d6f114681` "veilweaver verticle slice second commit" added it, but `astraweave_audio` was never `use`d in the demo's source). Stale residue, or planned wiring that never landed? Andrew's call.

- **Parked scripting↔audio integration:** `astraweave-scripting/src/lib.rs:536-547` carries a TODO comment block titled `TODO(scripting-audio-integration)` that lays out a sequence (add dep, create `AudioEngineResource` wrapper, `world.get_resource_mut::<AudioEngineResource>()`, call `play_sfx_file`/`play_sfx_3d_file`). The comment claims "Add astraweave-audio to Cargo.toml dependencies (already added)" but workspace grep confirms `astraweave-scripting/Cargo.toml` does **not** depend on `astraweave-audio` as of `a2474c5b7`, and no `AudioEngineResource` type exists in the codebase. Resolution depends on the Send/Sync question above — wrapping AudioEngine in any ECS Resource shape requires Send + Sync, which the current rodio/cpal stack precludes. Andrew's call.

- **`EmitterId` allocator-less design:** `EmitterId` is `u64` (`engine.rs:9`) with no allocator and no "no-emitter" sentinel. Two callers picking the same value silently share a `SpatialSink` and overwrite each other's positions (see §6 trap). Current callers in production code: editor uses `rand::random::<u64>()` (`audio_bridge.rs:192`) — high-entropy, collision-unlikely; NPC runtime uses hardcoded `10` for guards (`runtime.rs:49`); demos use small integers 1, 42, 100, 101, 102, etc. (`examples/audio_spatial_demo/src/main.rs:119-148`, `examples/npc_town_demo`). There is no documented coordination scheme. Is the design intent "any caller picks any u64, collisions are caller responsibility", "use entity ID as emitter ID", or "the engine should manage allocation"? No code, doc, or comment provides guidance. Andrew's call.

---

## 12. Maintenance Notes

**Update this doc when:**
- The bus count or names in `AudioEngine` change (§1 status note, §3 vocabulary, §8 invariants 1-3).
- A new `AudioAction` variant is added or an existing one moves from "no-op" to "implemented" (§5 file map, §6 forward-designed table, §11 open questions).
- The dialogue resolution chain (`speak_current`) gains or loses a tier (§2 pipeline, §8 invariant 11).
- An ECS integration is introduced (§4 cross-system, §7 decision log).
- rodio version is bumped (§7 first decision).
- The aspirational `docs/src/core-systems/audio.md` is rewritten to match reality (§6 coexisting abstractions row, §11 question).
- `PanMode` is wired into actual logic, or removed (§6 trap, §11 question).
- TTS temp-file cleanup is implemented (§9 resource ownership note, §11 question).

**Verification process:**
- Spot-check: `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-audio/src/` should show roughly the surface listed in §3 vocabulary and §5 file map.
- `cargo tree -p astraweave-audio --depth 1` should list rodio, astraweave-gameplay, glam, anyhow, rand, serde, serde_json, thiserror, toml. Anything more or less indicates dependency drift since §7 first decision.
- `rg 'astraweave_audio' --type rust -g '!*test*' -g '!benches/*'` should find the consumers listed in §4. New consumers must be added to that table.
- Stamp the new commit hash and date in the metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. The crate is **not** an ECS-citizen — there is no audio system stage and no `Resource`. Consumers own `AudioEngine` directly. Don't try to look it up from a `World`.
2. The editor `AudioPanel` exposes 10+ controls the engine doesn't actually implement (HRTF, Doppler, distance model, reverb, crossfade duration storage, shuffle/loop). These are forward-designed UI placeholders. `audio_bridge.rs:165-182` is the boundary.
3. `docs/src/core-systems/audio.md` and `docs/src/api/audio.md` describe an API that **does not exist** in the actual crate. Do not trust them as a source of API truth. The real API is the ~30 lines of `lib.rs` plus the source it re-exports.

**Files you'll most likely touch:**
- `astraweave-audio/src/engine.rs` — bus volumes, listener pose, spatial sinks, ducking
- `astraweave-audio/src/dialogue_runtime.rs` — dialogue voice resolution chain
- `tools/aw_editor/src/audio_bridge.rs` — editor wiring; the place where panel actions become engine calls
- `tools/aw_editor/src/panels/audio_panel.rs` — UI surface; add new `AudioAction` variants here

**Files you should NOT touch without strong reason:**
- `astraweave-audio/src/mutation_tests.rs` — mutation-resistance assertions; changes here often mask real bugs.
- The 14 integration test files — many gate on real fixture audio files; rewriting fixtures requires regenerating via `tests/test_asset_generator.rs`.

**Common mistakes when changing this system:**
- **Adding a new bus and forgetting `set_master_volume`'s cascade** (`engine.rs:206-213`). All bus base-volumes must be re-multiplied on master change; spatial sinks must also be updated.
- **Adding a new `AudioAction` variant without updating `audio_bridge.rs::process_actions`.** The enum is `#[non_exhaustive]` (`audio_panel.rs:357`); within the same crate (`tools/aw_editor`), the compiler does still enforce exhaustive matching, but the attribute means any downstream consumer (none exists today) would not be forced to handle new variants. The risk in practice is forgetting to wire the action through to the engine and ending up with another silent no-op — see §6 forward-designed table.
- **Assuming `active_emitter_count() == playing_emitter_count`.** It doesn't. See §6 traps.
- **Adding a `play_sfx_*` call expecting "stop all SFX" to also clear it.** `stop_music` and `stop_ambient` exist; there is no `stop_sfx` or `stop_all_3d`. The `EditorAudioBridge::stop_all` only stops music and ambient (`audio_bridge.rs:268-273`).
- **Wrapping `AudioEngine` in `Arc<RwLock<_>>`** to share it across threads. rodio types prevent this (§11 first question).
- **Trusting the crate docstring's "4-bus mixer" wording.** It's 5 buses. Read the code.
