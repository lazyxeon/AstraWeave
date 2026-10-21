# World Design

## Loomspire Isle Biome

- **Visual Identity**: Floating limestone spires pierced by luminous thread conduits and bioluminescent flora. Skybox locked to perpetual twilight with drifting aurora sheets.
- **Regions in Slice**:
  1. *Frayed Causeway*: Narrow bridge fragments suspended by unstable threads (tutorial area).
  2. *Echo Grove*: Dense forest of crystal-thread trees providing cover and interactive weave anchors.
  3. *Loom Crossroads*: Central plaza with three storm conduits, used for narrative choice moment.
  4. *Oathbound Courtyard*: Boss arena anchored by gravity pylons with variable hazards.

## Environmental Storytelling

- Thread resonance logs embedded in obelisks (interactive lore nodes) explain prior weavers’ attempts.
- Companion callouts reference shifting island topology, reinforcing dynamic world fiction.
- Ambient wildlife (thread wisps, gliding mantaforms) react to weaving actions, giving visual feedback.

## Level Flow & Navigation

- Linear critical path with optional side alcoves for Echo Shard pickups (max +3 shards to respect 30-minute scope).
- Fate-weaving ensures each traversal challenge has at least one alternate solution (e.g., rebuild bridge vs redirect updraft to glide).
- Fast-travel anchors disabled to keep runtime; hub unlock teased in outro cinematic.

## Technical Notes

- Level streaming relies on `astraweave-scene` cell loading triggered by trigger volumes between regions.
- Atmospheric effects (storm reroute vs stabilization) toggle Post-FX profiles defined in `materials/loomspire/post_fx.toml` (to be authored).