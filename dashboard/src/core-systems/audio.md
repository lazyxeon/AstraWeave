<!--
  Audio System page — replaced 2026-05-15 as part of the post-trace-campaign
  reconciliation.
  Source: ARCHITECTURE_MAP.md §7.1 (Documentation Hazards) and `audio.md` trace.
  Pre-trace version (commit 28bc94f21, 2025-09-08, automated documentation pass)
  described AudioConfig, AudioBackend, BackendType, AudioListener, SpatialSound,
  AttenuationModel, ReverbZone, AudioOcclusion, MusicManager, MusicLayer,
  SfxManager, SoundPool, AudioMixer, Bus — none of which exist in
  astraweave-audio/src/lib.rs re-exports. The actual public surface is ~30 lines:
  AudioEngine, DialoguePlayer, VoiceBank, EmitterId, ListenerPose.
-->

# Audio System

```admonish warning title="Documentation under reconciliation"
This page was rewritten on 2026-05-15 to reflect the engineering reality surfaced by the
architecture trace campaign. A prior version (added in commit 28bc94f21,
2025-09-08, by an automated documentation pass) described an extensive `AudioConfig` /
`AudioBackend` / `MusicManager` / `SfxManager` / `AudioMixer` API surface that
**does not exist in the codebase**. The actual public surface is small (~30 lines of
re-exports). See the trace below for evidence-grounded detail.
```

## Actual public surface

<!-- Source: audio.md trace §1, §5 -->

The `astraweave-audio` crate is a focused facade over `rodio` / `cpal`. The `lib.rs`
re-exports are:

* `AudioEngine` — owns the rodio `OutputStream` chain. **`!Send + !Sync`** because
  `cpal::Stream` is non-Send-Sync across all platforms. Consequence: cannot be an
  ECS Resource, cannot be wrapped in `Arc<RwLock<_>>` for cross-thread sharing.
  Consumers hold it directly.
* `DialoguePlayer` — voice-line playback.
* `VoiceBank` — voice-line collection.
* `EmitterId` — `u64`, with no allocator and no sentinel. Collisions silently
  merge `SpatialSink`s (documented hazard, `audio.md` §6 trap).
* `ListenerPose` — ear-position pose for spatial panning.

## Bus layout

<!-- Source: audio.md §1 status note — the lib.rs:8 docstring is stale -->

`AudioEngine` mixes through **5 buses**: `master`, `music`, `ambient`, `voice`, `SFX`.
The `lib.rs:8` doc-comment that advertises "4-bus mixer (master, music, SFX, voice)"
is stale — the ambient bus was added in commit `745c100a8` alongside biome material
work. The bus count is documented in `audio.md` §1 as a status note.

## Editor integration

<!-- Source: audio.md §6 (10+ no-op AudioAction variants) and aw_editor.md §6 -->

The visual editor's `AudioPanel` exposes ~25 audio control knobs (HRTF, Doppler,
distance model, reverb, crossfade duration, shuffle/loop). **10+ of these have
bodyless or comment-only match arms in `tools/aw_editor/src/audio_bridge.rs:165-205`** —
they are forward-design UI placeholders that update editor state but do not reach
the runtime audio engine. Tracked as Q22 in `ARCHITECTURE_MAP.md` §14.

Specific documented no-ops:

* `pan_mode` field is stored and updated but **never read** by any other method;
  spatial sinks already created continue using rodio's spatial panning regardless.
* HRTF / distance-model / reverb knobs accept user input that the engine does not
  consume.

## Aspirational documentation (`docs/src/`)

Per `audio.md` §6 and `ARCHITECTURE_MAP.md` §7.1, the prior version of this page —
along with `docs/src/api/audio.md` — referenced a comprehensive audio
backend / manager API. Treat any older references to `AudioConfig`,
`AudioBackend`, `BackendType`, `AudioListener`, `SpatialSound`, `AttenuationModel`,
`ReverbZone`, `AudioOcclusion`, `MusicManager`, `MusicLayer`, `SfxManager`,
`SoundPool`, `AudioMixer`, or `Bus` as historical aspirational documentation. Those
types are not in `astraweave-audio/src/lib.rs` re-exports.

## Where to actually look in the code

| Need | File |
|------|------|
| Public re-exports | `astraweave-audio/src/lib.rs` |
| Audio engine implementation | `astraweave-audio/src/` |
| Editor bridge (10+ no-op action variants) | `tools/aw_editor/src/audio_bridge.rs:165-205` |
| Editor audio panel UI | `tools/aw_editor/src/panels/audio_panel.rs` |

## Further reading

* [`audio.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/audio.md) — full audio-system trace (file map, conflict map, decision
  log, invariants, open questions).
* [`ARCHITECTURE_MAP.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/ARCHITECTURE_MAP.md) §7.1 — documentation-hazard inventory.
* **Interactive workspace map** — select `astraweave-audio` to see the
  panel detail with dormancy evidence and the §4.3 silent-failure shapes.
