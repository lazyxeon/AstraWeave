# Architecture Trace: Persistence-ECS (Save/Load + ECS Integration)

> **Scope note:** This doc traces the **third** of three subsystems CLAUDE.md groups under "Networking": the save/load + replay persistence layer. The other two are:
> - `docs/architecture/net.md` — `astraweave-net` (snapshot-based game server)
> - `docs/architecture/net_ecs.md` — `astraweave-net-ecs` + standalone matchmaking trio

## Metadata

| Field | Value |
|---|---|
| **System name** | Persistence-ECS (save / load / migrate / replay) |
| **Primary crates** | `astraweave-persistence-ecs` (ECS Plugin + world serialization + hashing); `persistence/aw-save` (file format, atomic I/O, versioning, migration) |
| **Document version** | 1.2 |
| **Last verified against commit** | `a2474c5b7` |
| **Last verified date** | 2026-05-12 |
| **Status** | Active (mixed maturity: file format + serialization + hashing are production-grade; ECS Plugin layer's `auto_save_system` and replay event apply are TODO stubs) |
| **Revision history** | 1.2 (2026-05-12): Deep investigation pass. **Closed Open Question 13** (CLI tools location) — purely factual question, comprehensively answered (`tools/aw_save_cli/` exists, was created in the same commit as `aw-save` itself). Resolution moved to §4 / §6. **New emergent finding**: a separate sibling crate `crates/astraweave-persistence-player` exists for player-profile persistence (TOML-based, distinct data model). Has zero dependency relationship with this subsystem; only a single doc-comment reference. Added to §6 as a coexisting-but-disjoint abstraction so future agents don't conflate them. Enriched §11 Q1, Q2, Q11 with commit-date provenance: the `astraweave-stress-test` declaration of both `astraweave-net-ecs` and `astraweave-persistence-ecs` and the `astraweave-memory` declaration in `astraweave-persistence-ecs/Cargo.toml` all landed in the **same commit** `08befc6ec` (2025-10-01, "phase 6 implementation"); test artifacts (`slot*.awsv`, `savegame.bin`) were committed in unrelated subsequent commits with misleading titles.<br><br>1.1 (2026-05-12): Verification pass. **Major corrections**: (1) The §4 `[INFERRED]` claim that CLI tools "may live elsewhere or be planned" was wrong — `tools/aw_save_cli/src/main.rs` (110 lines, clap-based) is a real binary that consumes `aw-save`. Added to §4 downstream table; resolved the marker. (2) Discovered second `aw-save` consumer not previously documented: `examples/save_integration/src/main.rs` (234 lines) imports `aw-save` types directly. Added to §4. (3) Resolved §9 `[INFERRED]` about auto-pruning absence — direct grep `fs::remove\|remove_file\|tokio::fs::remove persistence/aw-save/src` returns zero matches, conclusively confirming no auto-prune path exists. (4) Verified `Entity::to_raw()` layout claim in §3 against actual source at `astraweave-ecs/src/entity_allocator.rs:95-103` — claim is exact. |
| **Owner notes** | `aw-save` (file format and SaveManager) is fully production-shaped: atomic writes via tmp+fsync+rename, CRC32 + LZ4, explicit V1→V2 migration, 256 slots per player. `astraweave-persistence-ecs` (ECS integration) is partly stub: the ECS `auto_save_system` function body is a comment-only TODO; replay event application is a TODO; `CPersistenceManager::save_game` hardcodes inventory (`credits: 1000`, `items: []`) and emits zero companions. **`astraweave-stress-test/Cargo.toml:21` declares `astraweave-persistence-ecs` as a dep but no source file imports anything from it** — same pattern as `net-ecs` (declared-but-unused). The companion `astraweave-memory` workspace dep declared at `astraweave-persistence-ecs/Cargo.toml:20` is also never imported in the crate's source. |

---

## 1. Executive Summary

**What this system does:**
Persists AstraWeave's ECS world state to disk with crash-safety, versioning, compression, and integrity checks; reloads it back into a fresh ECS world; supports cross-version save migration; and provides scaffolding for deterministic replay. Combines two layered crates:

1. **`aw-save`** — the on-disk format and atomic I/O primitives (file extension `.awsv`, magic `ASVS`, CRC32 + LZ4, `SaveBundleV2` schema, explicit V1→V2 migration, 256-slot-per-player layout).
2. **`astraweave-persistence-ecs`** — the ECS adapter: `PersistencePlugin`, `CPersistenceManager` / `CReplayState` components, two systems (`auto_save_system`, `replay_system`), and three world-blob functions (`serialize_ecs_world`, `deserialize_ecs_world`, `calculate_world_hash`).

**Why it exists:**
Saving game state across crashes and version upgrades is hard. The two-layer split separates "what bytes go on disk" (`aw-save`, file format) from "how an ECS world becomes bytes" (`astraweave-persistence-ecs`, world serialization). The `WorldState.ecs_blob: Vec<u8>` field is the integration seam — `aw-save` treats it as opaque; `astraweave-persistence-ecs` produces and consumes it.

**Where it primarily lives:**
- `persistence/aw-save/src/lib.rs` — 403 lines. `SaveManager`, `SaveBundleV2`, `WorldState`, `PlayerInventory`, `ItemStack`, `CompanionProfile`, `SaveMeta`, `SaveBundleV1` (migration), `write_awsv`/`read_awsv`/`read_any_version` (file format), `SAVE_SCHEMA_VERSION = 2`, `MAGIC = "ASVS"`.
- `astraweave-persistence-ecs/src/lib.rs` — 1292 lines. `PersistencePlugin`, `CPersistenceManager`, `CReplayState`, `ReplayEvent`, `SaveMetadata`, `SerializedEntity`, `SerializedWorld`, `auto_save_system`, `replay_system`, `serialize_ecs_world`, `deserialize_ecs_world`, `calculate_world_hash`. Most of the line count is heavily-annotated doc comments on the three exported pipeline functions (performance projections, 60-FPS impact, hash-collision analysis).
- `persistence/aw-save/tests/`, `astraweave-persistence-ecs/tests/` — 179 tests across the two crates (135 in persistence-ecs, 44 in aw-save).
- `persistence/aw-save/index.json`, `slot00_*.awsv`, ... — test artifacts checked into the repo (5 `.awsv` files + `index.json`).
- `astraweave-persistence-ecs/savegame.bin` — 2-byte test artifact at the crate root.

**Status note:**
The disk-format layer (`aw-save`) is fully production-grade and CI-covered. The ECS integration layer (`astraweave-persistence-ecs`) has working roundtrip serialization but several systems and helpers are TODO-stubs (see §6 cognitive traps). **No production code outside the crates' own tests uses `astraweave-persistence-ecs`.** It is scaffolding ready to be wired.

---

## 2. Authoritative Pipeline

```text
[ECS World — astraweave_ecs::World with entities + components]
    │
    │ serialize_ecs_world(&world)                          (persistence-ecs/lib.rs:278-366)
    ▼
[Entity discovery (10 separate Query<C> passes)]
    │
    │ Query::<CPos>::new(world)  → insert entity into HashSet<Entity>
    │ Query::<CHealth> ...
    │ Query::<CTeam> ...
    │ Query::<CAmmo> ...
    │ Query::<CCooldowns> ...
    │ Query::<CDesiredPos> ...
    │ Query::<CAiAgent> ...
    │ Query::<CPersona> ...
    │ Query::<CMemory> ...
    │ (Note: CLegacyId is read in the per-entity collect pass at :352 but is NOT used to seed entity_set)
    │
    ▼
[Per-entity collect into SerializedEntity]
    │
    │ for entity in entity_set:
    │     SerializedEntity {
    │         entity_raw: entity.to_raw(),     // u64 packing id + generation (lib.rs:344)
    │         pos: world.get::<CPos>(entity).copied(),
    │         health, team, ammo, cooldowns, desired_pos, ai_agent, legacy_id, persona, memory: ...
    │     }
    │
    ▼
[SerializedWorld { entities: Vec<SerializedEntity>, world_tick: 0 }]
    │  ↑ world_tick HARDCODED to 0 — TODO at lib.rs:361 ("Get from world state when available")
    │
    │ postcard::to_allocvec(&serialized_world)            (lib.rs:365)
    ▼
[Vec<u8> ECS blob]
    │
    │ This blob becomes WorldState.ecs_blob in aw-save's bundle:
    │
    │ CPersistenceManager::save_game(slot, world_tick, world_hash, ecs_blob)  (persistence-ecs/lib.rs:113-153)
    │
    │   bundle = SaveBundleV2 {
    │       schema: SAVE_SCHEMA_VERSION (= 2),
    │       save_id: Uuid::new_v4(),
    │       created_at: OffsetDateTime::now_utc(),
    │       player_id: self.current_player.clone(),
    │       slot,
    │       world: WorldState { tick, ecs_blob, state_hash },
    │       companions: vec![],          // TODO at lib.rs:121
    │       inventory: PlayerInventory { credits: 1000, items: vec![] },   // both TODO at lib.rs:125-126
    │       meta: { "engine_version" → CARGO_PKG_VERSION },
    │   }
    │
    ▼
[aw-save::SaveManager::save(player_id, slot, bundle)]    (aw-save/lib.rs:53-70)
    │
    │ 1. fs::create_dir_all(<root>/<sanitized(player_id)>/)
    │ 2. Windows-safe timestamp: replace ':' with '-'
    │ 3. Filename: slot{NN}_{stamp}_{save_id}.awsv
    │
    ▼
[write_awsv(path, &bundle)]                              (aw-save/lib.rs:258-288)
    │
    │ 1. postcard::to_allocvec(v2)
    │ 2. lz4_flex::compress_prepend_size(payload)
    │ 3. CRC32 over compressed payload
    │ 4. Header: magic[4]="ASVS" + version u16 + codec u8(=1) + reserved u8(=0)
    │             + data_len u32 + crc32 u32
    │ 5. Atomic write: open path.tmp, write_all, sync_all, close, then fs::rename(path.tmp, path)
    │
    ▼
[On-disk .awsv file]
    │
    │ Side effect: write_or_update_index(dir, &bundle, &path)  (aw-save/lib.rs:210-229)
    │   → Read existing index.json (if any), upsert SaveMeta by save_id,
    │     sort by (slot, created_at), write back as pretty JSON

──────────────────────────────────────────────────────────────────────
Load path:
    │
    │ CPersistenceManager::load_game(slot) → (SaveBundleV2, PathBuf)   (persistence-ecs/lib.rs:155-159)
    ▼
[SaveManager::load_latest_slot(player_id, slot)]         (aw-save/lib.rs:72-93)
    │
    │ 1. fs::read_dir(<player_dir>)
    │ 2. Filter to *.awsv files starting with "slot{NN}_"
    │ 3. Lexicographic sort (timestamp embedded in filename → newest wins)
    │ 4. Take the last entry
    │
    ▼
[read_awsv(path)]                                        (aw-save/lib.rs:290-300)
    │
    │ Delegates to read_any_version(path):
    │   - Verify magic == "ASVS"
    │   - Read version u16 + codec u8 + reserved u8 + data_len u32 + crc32 u32
    │   - Read payload bytes
    │   - Verify CRC32 over payload bytes (bail if mismatch)
    │   - Decompress: lz4_flex::decompress_size_prepended(payload)  if codec == 1
    │
    │ Then dispatch on version:
    │   2 → postcard::from_bytes::<SaveBundleV2>(blob)
    │   1 → postcard::from_bytes::<SaveBundleV1>(blob)  →  v1.into_v2()
    │   other → bail!("unknown save version {other}")
    │
    ▼
[SaveBundleV2 reconstructed]
    │
    │ Caller (e.g. game code) extracts bundle.world.ecs_blob and calls:
    │
    │ deserialize_ecs_world(&ecs_blob, &mut world)        (persistence-ecs/lib.rs:446-504)
    │
    │   if ecs_blob.is_empty(): return Ok(())             (lib.rs:447-450 — silent no-op)
    │   else:
    │     - postcard::from_bytes::<SerializedWorld>(blob)
    │     - First pass: spawn N new entities, build HashMap<u64, Entity> id_map
    │     - Second pass: for each SerializedEntity:
    │         · world.insert<CPos|CHealth|...>(new_entity, ...)
    │
    ▼
[Restored ECS world with fresh entity IDs]
    │  ↑ Entity IDs are NEW — the saved id_map → new_entity translation breaks
    │  ↑ stable cross-save references unless callers walk CLegacyId / similar.
    │  ↑ The function doesn't remap CLegacyId itself — comment at lib.rs:491 says
    │  ↑ "Insert as-is (may need entity ID translation logic)"

──────────────────────────────────────────────────────────────────────
Hash path (for integrity check / replay / cheat detection):
    │
    │ calculate_world_hash(&world)                        (persistence-ecs/lib.rs:612-663)
    ▼
[Entity discovery (only 2 component types: CPos OR CHealth)]
    │
    │ entity_list = []
    │ Query<CPos>: push every entity
    │ Query<CHealth>: push if not already in list
    │ entity_list.sort_unstable()
    │
    │ Note: CTeam-only / CAmmo-only / CCooldowns-only / etc. entities
    │ that lack BOTH CPos and CHealth are NOT included in the hash.
    │
    ▼
[Per-entity hashing into DefaultHasher (SipHash-1-3)]
    │
    │ for entity in entity_list:
    │     entity.hash(&mut hasher)
    │     if CPos:   hash pos.x, pos.y
    │     if CHealth: hash hp
    │     if CTeam:   hash id
    │     if CAmmo:   hash rounds
    │     (CCooldowns, CDesiredPos, CAiAgent, CLegacyId, CPersona, CMemory NOT hashed)
    │     ↑ TODO at lib.rs:596 — "Add CCooldowns, CAiAgent, CPersona, CMemory for complete coverage"
    │
    ▼
[u64 hash via hasher.finish()]

──────────────────────────────────────────────────────────────────────
Migration path (legacy V1 → current V2):
    │
    │ CPersistenceManager::migrate_save(path, resave)     (persistence-ecs/lib.rs:179-181)
    ▼
[SaveManager::migrate_file_to_latest(path, resave)]     (aw-save/lib.rs:101-115)
    │
    │ 1. read_any_version(path) → AnySave { version, blob }
    │ 2. match version:
    │      1 → postcard::from_bytes::<SaveBundleV1>(blob) → v1.into_v2()    (aw-save/lib.rs:182-196)
    │           · companions = self.companion.into_iter().collect()    (Option<Companion> → Vec)
    │           · save_id = Uuid::new_v4()                              (generated fresh)
    │           · schema = SAVE_SCHEMA_VERSION                          (bumped to 2)
    │      2 → already current
    │      other → bail
    │ 3. if resave: write_awsv(path, &v2)
    │ 4. return Ok(v2)

──────────────────────────────────────────────────────────────────────
Replay path (scaffolded, partial):
    │
    │ CPersistenceManager::start_replay(slot)             (persistence-ecs/lib.rs:161-171)
    ▼
[Load bundle from slot]
    │
    │ Return CReplayState {
    │   is_replaying: true,
    │   current_tick: 0,
    │   total_ticks: bundle.world.tick,
    │   events: vec![],                  // TODO at lib.rs:169 — "Load replay events from save data"
    │ }
    │
    ▼
[PersistencePlugin::build adds replay_system to "pre_simulation" stage]   (lib.rs:67)
    │
    │ replay_system(world):                              (lib.rs:78-104)
    │   For every entity with CReplayState where is_replaying:
    │     if current_tick < total_ticks:
    │       // TODO: Implement replay event application   (lib.rs:96)
    │       current_tick += 1
    │     else:
    │       is_replaying = false
    │
    │ The system advances the tick counter but does NOT apply any saved events.

──────────────────────────────────────────────────────────────────────
Auto-save path (scaffolded, empty body):
    │
    │ PersistencePlugin::build adds auto_save_system to "post_simulation" stage  (lib.rs:66)
    ▼
[auto_save_system(_world): { /* TODO body */ }]    (lib.rs:72-75)
    │
    │ Function takes &mut World but currently does nothing.
    │ Comment: "Query for persistence manager and save at intervals"
```

### Stage-by-stage detail

#### Stage 1: ECS world → `SerializedWorld` (`persistence-ecs/lib.rs:278-366`)
**Role:** Walk all ECS entities that carry any of 10 supported components and produce a postcard-encodable struct.
**Inputs:** `&World`.
**Outputs:** `Result<Vec<u8>>` (postcard-encoded `SerializedWorld`).
**Notes:**
- Entity discovery does **9 separate `Query<C>` passes** to populate a `HashSet<Entity>`. The 10th component, `CLegacyId`, is **not** in the discovery loop — entities holding only `CLegacyId` won't be saved. (`CLegacyId` IS collected in the per-entity inner loop at `lib.rs:352`, so entities discovered via any of the other 9 still get their `CLegacyId` saved.)
- `entity_raw: entity.to_raw()` packs Entity's (id, generation) into a u64 (`lib.rs:344` + doc comment at `lib.rs:233-238`).
- `SerializedWorld.world_tick` is **hardcoded to 0** with `// TODO: Get from world state when available` (`lib.rs:361`). The `CPersistenceManager::save_game` function does pass a `world_tick: u64` parameter through, but it lands in `WorldState.tick` (the aw-save side), not in the ECS-side `SerializedWorld.world_tick`.

#### Stage 2: `SaveBundleV2` construction (`persistence-ecs/lib.rs:113-153`)
**Role:** Build the wire-shaped save bundle that `aw-save` understands.
**Inputs:** `slot, world_tick, world_hash, ecs_blob`.
**Outputs:** `Result<PathBuf>` (final on-disk path).
**Notes:** The companion list is hardcoded empty (`lib.rs:121`: `// TODO: Query ECS for companion data`); the inventory is `PlayerInventory { credits: 1000, items: vec![] }` (`lib.rs:124-127`, both fields tagged TODO). Only the `engine_version → CARGO_PKG_VERSION` meta entry is populated. Callers cannot supply richer save data through this API as of `a2474c5b7`.

#### Stage 3: `aw-save` atomic write (`aw-save/lib.rs:258-288`)
**Role:** Write the bundle to disk with crash-safety, compression, and integrity.
**Inputs:** `&Path`, `&SaveBundleV2`.
**Outputs:** `Result<()>`.
**Notes:**
- Sequence: postcard encode → lz4 compress (size-prepended) → CRC32 over compressed bytes → header pack → open `path.tmp` → `write_all` → `sync_all` → `fs::rename(tmp, path)`.
- The order matters: the `fs::rename` is the atomic commit point. If the process crashes before `rename`, the old `.awsv` is intact and only a stray `.tmp` is left.
- `path.with_extension("tmp")` (line 276) means a file called `slot00_foo.awsv` writes to `slot00_foo.tmp` first — same directory, so `rename` is guaranteed atomic on POSIX and on NTFS.
- No `fsync` on the parent directory after rename. [INFERRED — depending on filesystem semantics, a crash immediately after `rename` but before directory-entry persistence could lose the new file on some filesystems; the code does not flush the directory.]

#### Stage 4: `aw-save` atomic read + CRC verify + LZ4 decompress (`aw-save/lib.rs:307-338`)
**Role:** Validate integrity, decompress, and dispatch on schema version.
**Inputs:** `&Path`.
**Outputs:** `Result<AnySave { version, blob }>`.
**Notes:**
- Bails on bad magic (`lib.rs:311-313`).
- Bails on CRC mismatch (`lib.rs:326-328`).
- Bails on unknown codec — only `CODEC_LZ4 = 1` is recognized (`lib.rs:330-333`).
- Version dispatch happens in the caller (`read_awsv` at `lib.rs:290-300`); `read_any_version` returns the raw decompressed blob plus the version byte.

#### Stage 5: V1 → V2 migration (`aw-save/lib.rs:170-196`)
**Role:** Translate a `SaveBundleV1` (legacy single-companion shape) into the current `SaveBundleV2` (Vec-of-companions shape, with a fresh `save_id` Uuid).
**Inputs:** `SaveBundleV1`.
**Outputs:** `SaveBundleV2`.
**Notes:**
- `companions = self.companion.into_iter().collect()` (`lib.rs:191`) maps `Option<CompanionProfile>` → `Vec<CompanionProfile>` (empty vec if `None`).
- The migration **generates a fresh `Uuid::new_v4()`** (`lib.rs:186`) — the migrated save has a new ID, not the old one. If callers use `save_id` for deduplication, they need to remember this.
- `created_at` is preserved from the original V1.

#### Stage 6: ECS deserialization with ID remapping (`persistence-ecs/lib.rs:446-504`)
**Role:** Spawn fresh ECS entities and insert all saved components, remapping old entity IDs to new ones.
**Inputs:** `&[u8]`, `&mut World`.
**Outputs:** `Result<()>`.
**Notes:**
- Empty blob is a **silent no-op** (`lib.rs:447-450`: `if ecs_blob.is_empty() { return Ok(()) }`).
- Two-pass design: first pass `world.spawn()` per saved entity and build `HashMap<u64, Entity>`; second pass `world.insert(remapped_entity, component)`.
- `CLegacyId` is **inserted as-is** (`lib.rs:490-493`) with comment `// Remap entity reference in CLegacyId if needed / For now, insert as-is (may need entity ID translation logic)`. If `CLegacyId` carries a reference to another entity by id, that reference is **not** remapped through the id_map.

#### Stage 7: Deterministic world hash (`persistence-ecs/lib.rs:612-663`)
**Role:** Produce a `u64` hash of the current ECS world state for validation / replay verification.
**Inputs:** `&World`.
**Outputs:** `u64`.
**Notes:**
- Uses `std::collections::hash_map::DefaultHasher` (currently SipHash-1-3 per the rustc default; see doc comment at `lib.rs:536`).
- **Only entities with `CPos` OR `CHealth`** are included in the hash (`lib.rs:619-633`). Entities holding only `CTeam`, `CAmmo`, etc. are silently excluded.
- **Only 4 component types are hashed**: `CPos.{x, y}`, `CHealth.hp`, `CTeam.id`, `CAmmo.rounds` (`lib.rs:637-660`). Six other supported components (`CCooldowns`, `CDesiredPos`, `CAiAgent`, `CLegacyId`, `CPersona`, `CMemory`) are NOT hashed. The function's own doc comment acknowledges this at `lib.rs:596`: `**TODO**: Add CCooldowns, CAiAgent, CPersona, CMemory for complete coverage`.
- `entity_list.sort_unstable()` ensures deterministic iteration order (`lib.rs:634`).
- Documented as cryptographically weak ("DO NOT use for security", `lib.rs:537`). The hash is for integrity / replay validation only.

#### Stage 8: ECS Plugin systems (`persistence-ecs/lib.rs:63-104`)
**Role:** Wire `auto_save_system` and `replay_system` into the App's stage scheduler.
**Inputs:** `&mut App`.
**Outputs:** Two systems registered.
**Notes:**
- `PersistencePlugin.save_directory: PathBuf` is stored on the plugin struct but **never read** by `build()`. The field is `#[allow(dead_code)]` (`lib.rs:52-55`). Setting up a real `SaveManager` would require the caller to do that work outside the plugin.
- `auto_save_system(_world)` body is comment-only (`lib.rs:72-75`). The system runs in `"post_simulation"` and does nothing.
- `replay_system(world)` increments `CReplayState.current_tick` per call but the actual event application is `// TODO: Implement replay event application` (`lib.rs:96`).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **SAVE_SCHEMA_VERSION** | `u16 = 2` — the current bundle version. V1 exists for migration; older versions error out. | `aw-save/lib.rs:31` |
| **MAGIC** | `b"ASVS"` — 4-byte file-format identifier. | `aw-save/lib.rs:28` |
| **CODEC_LZ4** | `u8 = 1` — the only currently-recognized payload codec byte. | `aw-save/lib.rs:29` |
| **.awsv** | File extension for AstraWeave Save Versioned files. | `aw-save/lib.rs:13, 79` (filter), `lib.rs:64` (filename) |
| **SaveManager** | Public API rooted at a base directory. Operates on `<root>/<sanitized(player_id)>/slot{NN}_{timestamp}_{uuid}.awsv`. | `aw-save/lib.rs:34-116` |
| **SaveBundleV2** | The current bundle shape: `{ schema, save_id, created_at, player_id, slot, world, companions, inventory, meta }`. | `aw-save/lib.rs:120-132` |
| **WorldState** | `{ tick: u64, ecs_blob: Vec<u8>, state_hash: u64 }`. The `ecs_blob` is the integration seam — opaque to aw-save, owned by the ECS layer. | `aw-save/lib.rs:135-142` |
| **SaveMeta** | Per-save index entry: `{ save_id, file, created_at, player_id, slot, schema }`. Stored in `index.json` and produced from directory scans as fallback. | `aw-save/lib.rs:201-208` |
| **index.json** | Per-player JSON file listing all `SaveMeta` entries, sorted by `(slot, created_at)`. Rebuilt from disk scan if missing or unreadable. | `aw-save/lib.rs:210-254` |
| **CPersistenceManager** | ECS component wrapping `{ save_manager: SaveManager, current_player: String }`. | `persistence-ecs/lib.rs:18-21` |
| **PersistencePlugin** | ECS Plugin holding `save_directory: PathBuf` (unused) that registers `auto_save_system` + `replay_system`. | `persistence-ecs/lib.rs:52-69` |
| **SerializedEntity** | Per-entity serialization shape: `entity_raw: u64` (packed id+generation) + 10 `Option<C>` component fields. | `persistence-ecs/lib.rs:185-198` |
| **SerializedWorld** | `{ entities: Vec<SerializedEntity>, world_tick: u64 }`. Note: `world_tick` is hardcoded `0` at write time. | `persistence-ecs/lib.rs:201-205` |
| **CReplayState** | ECS component: `{ is_replaying: bool, current_tick: u64, total_ticks: u64, events: Vec<ReplayEvent> }`. | `persistence-ecs/lib.rs:36-41` |
| **ReplayEvent** | `{ tick: u64, event_type: String, data: Vec<u8> }`. Opaque payload. | `persistence-ecs/lib.rs:44-49` |
| **SaveMetadata** | Distinct from `aw-save::SaveMeta`. ECS-layer struct: `{ player_id, slot, save_id, created_at, world_tick, world_hash }`. Used as a serializable summary, not for indexing. | `persistence-ecs/lib.rs:24-32` |
| **entity_raw** | Stable `u64` packing of an `astraweave_ecs::Entity` via `entity.to_raw()`. Layout: `(id as u64) | ((generation as u64) << 32)`. | `persistence-ecs/lib.rs:233-238, 344` |

### Terms to NOT confuse

- **`aw-save::SaveMeta` (file index entry) vs. `persistence-ecs::SaveMetadata` (ECS-layer summary):** Different fields, different purposes. `SaveMeta` lives in `index.json` and is keyed by `save_id`. `SaveMetadata` is a higher-level summary type that includes `world_tick` and `world_hash` — fields `SaveMeta` does not carry.
- **`aw-save::WorldState.tick` vs. `persistence-ecs::SerializedWorld.world_tick`:** The first is the authoritative tick stored at the bundle level. The second is supposed to be the ECS-layer tick but is **hardcoded to 0** at serialization time (`lib.rs:361`). `CPersistenceManager::save_game` passes `world_tick` to the bundle's `WorldState.tick` — that's the only place the actual tick ends up.
- **`aw-save::WorldState.state_hash` vs. the `world_hash` parameter to `save_game`:** They're meant to be the same value, plumbed through. `save_game(slot, world_tick, world_hash, ecs_blob)` writes `world_hash` into `WorldState { state_hash: world_hash }` (`persistence-ecs/lib.rs:145`).
- **`SerializedEntity.entity_raw` (u64) vs. `astraweave_ecs::Entity`:** The serialized form is `u64`. `Entity::to_raw()` and the new entity from `world.spawn()` are bridged through the `id_map` HashMap during deserialization.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-ecs` | `App`, `Plugin`, `Query<C>`, `Entity::to_raw()`, `World::spawn() / get / get_mut / insert` (`persistence-ecs/lib.rs:9`) | Generic ECS-shaped reads and writes | `serialize_ecs_world` and `deserialize_ecs_world` are the only non-Plugin consumers of the ECS API; both take `&World` or `&mut World` directly. |
| `astraweave-core::ecs_components` | `CPos`, `CHealth`, `CTeam`, `CAmmo`, `CCooldowns`, `CDesiredPos`, `CAiAgent`, `CLegacyId`, `CPersona`, `CMemory` (`persistence-ecs/lib.rs:8`) | All 10 supported component types | The crate is tightly coupled to this specific component set. Adding a new component to the engine requires touching `SerializedEntity`, both passes of `serialize_ecs_world`, `deserialize_ecs_world`, and (optionally) `calculate_world_hash`. |
| `aw-save` (path dep, `persistence-ecs/Cargo.toml:17`) | `SaveBundleV2`, `SaveManager`, `WorldState`, `SAVE_SCHEMA_VERSION`, `SaveMeta`, `PlayerInventory`, `CompanionProfile` | The on-disk bundle shape and the SaveManager facade | Used by `CPersistenceManager` and the migration helper. |
| `tempfile` (dev-dep at `Cargo.toml:23`) | `tempdir()` | Per-test scratch directories | Used in 28+ tests across both crates to avoid touching the real filesystem. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-stress-test` (Cargo.toml:21: `astraweave-persistence-ecs = { workspace = true }`) | none in source | none | **Declared but unused.** `grep -rn "use astraweave_persistence_ecs" astraweave-stress-test/` returns no source-file matches. Same pattern as `astraweave-net-ecs`'s unused declaration in the same crate. |
| Filesystem (`.awsv` files + `index.json`) | `aw-save::SaveManager::save / load_latest_slot / migrate_file_to_latest / list_saves` | One directory per player, multiple `.awsv` files per slot (timestamp-ordered) | The format is portable: same `.awsv` file can be moved between machines / OSes (Windows-safe timestamps, sanitized player_id). |
| `tools/aw_save_cli` (the CLI tools referenced in `aw-save/README.md`) | `use aw_save::{CompanionProfile, ItemStack, PlayerInventory, SaveBundleV2, SaveManager, WorldState, SAVE_SCHEMA_VERSION}` (`tools/aw_save_cli/src/main.rs:1-11`) | Command-line save/load/migrate operations | 110-line `clap`-based binary in `tools/aw_save_cli/`. Crate name `aw_save_cli`. The "CLI tools" mentioned in `aw-save/README.md` are not aspirational — they live at this path. Verified 2026-05-12. |
| `examples/save_integration` | `use aw_save::{CompanionProfile, ItemStack, PlayerInventory, SaveBundleV2, SaveManager, WorldState, SAVE_SCHEMA_VERSION}` (`examples/save_integration/src/main.rs:8-11`) | Demonstration of save integration with `astraweave-core::World` | 234-line example. Uses `aw-save` types directly, bypassing `astraweave-persistence-ecs`. |

### Bidirectional / Coupled

- **`CPersistenceManager.save_manager`** owns the on-disk root directory; every save/load call routes through it. The component is held inside the ECS world, so an ECS world without a `CPersistenceManager` entity cannot save through the standard path.
- **`CReplayState` ↔ `replay_system`**: The component holds the replay state; the system advances it. But event application is TODO, so the replay loop is currently a pure tick-counter.
- **`aw-save::WorldState.ecs_blob: Vec<u8>`** is the integration seam between the two crates. `aw-save` treats it as opaque (it's just bytes inside the postcard-encoded V2 bundle). `astraweave-persistence-ecs` is the only producer/consumer of those bytes.

### Documentation references with no code backing

- **None observed.** The `aw-save/README.md` "CLI tools" mention is backed by `tools/aw_save_cli/` (verified 2026-05-12). The crate's own `Cargo.toml` has no `[[bin]]` section, but the CLI lives as a separate `aw_save_cli` crate under `tools/` that depends on `aw-save` via path.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `persistence/aw-save/src/lib.rs` | All public types + atomic file format + V1→V2 migration + SaveManager | Active | 403 lines. `#![forbid(unsafe_code)]` (line 1). |
| `persistence/aw-save/tests/integration_test.rs` | Save/load roundtrip and corruption integration tests | Active (tests) | 3 tests. |
| `persistence/aw-save/tests/migration_test.rs` | V1→V2 migration tests | Active (tests) | 2 tests. |
| `persistence/aw-save/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness for aw-save | Active (tests) | 38 tests. |
| `persistence/aw-save/benches/save_benchmarks.rs` | Criterion benches for the file-format pipeline | Active | Run via `cargo bench -p aw-save`. |
| `persistence/aw-save/README.md` | Feature list + file format + usage examples | Active | Accurate for the file-format layer (matches code). |
| `persistence/aw-save/index.json` | Test artifact from running save tests | Active (test artifact) | Checked in alongside `.awsv` files; not git-ignored. |
| `persistence/aw-save/slot00_2026-03-13T08-45-25Z_*.awsv` × 2, `slot01_*.awsv`, `slot02_*.awsv`, `slot05_*.awsv` | Test artifacts | Active (test artifacts) | 5 `.awsv` files checked into the repo from a 2026-03-13 test run. |
| `astraweave-persistence-ecs/src/lib.rs` | ECS Plugin + components + 3 pipeline functions (`serialize_ecs_world`, `deserialize_ecs_world`, `calculate_world_hash`) + inline tests (28) | Active (mixed: Plugin systems are stubs; pipeline functions are production-grade) | 1292 lines. Most line count is doc comments. `#![forbid(unsafe_code)]` (line 1). |
| `astraweave-persistence-ecs/tests/save_load_tests.rs` | End-to-end save/load via `CPersistenceManager` | Active (tests) | 11 tests. |
| `astraweave-persistence-ecs/tests/corruption_recovery_tests.rs` | Corruption-detection scenarios (bad magic, bad CRC, truncated files) | Active (tests) | 13 tests. |
| `astraweave-persistence-ecs/tests/large_world_tests.rs` | Performance tests on large worlds | Active (tests) | 8 tests. |
| `astraweave-persistence-ecs/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness | Active (tests) | 64 tests. |
| `astraweave-persistence-ecs/tests/version_migration_tests.rs` | V1→V2 migration through the ECS layer | Active (tests) | 11 tests. |
| `astraweave-persistence-ecs/benches/persistence_ecs_benchmarks.rs` | Criterion benches for the pipeline functions | Active | 453 lines. |
| `astraweave-persistence-ecs/benches/world_serialization_benchmarks.rs` | Criterion benches for serialize/deserialize specifically | Active | 192 lines. |
| `astraweave-persistence-ecs/benches/persistence_ecs_adversarial.rs` | Worst-case-shape benches | Active | 1019 lines. |
| `astraweave-persistence-ecs/savegame.bin` | 2-byte test artifact | Active (test artifact) | Generated by a doc-example run; checked in. Cargo.toml doesn't ignore it. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care.
- **Active (tests)**: Carries no runtime weight but exercises invariants.
- **Active (mixed)**: Some parts production-grade, other parts stubbed.
- **Active (test artifact)**: Generated by a test run; checked into git rather than ignored.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `aw-save::SaveMeta` vs. `persistence-ecs::SaveMetadata` | `aw-save/lib.rs:201-208` vs. `persistence-ecs/lib.rs:24-32` | Two metadata types | `SaveMeta` is the on-disk index entry (`save_id, file, created_at, player_id, slot, schema`). `SaveMetadata` is a higher-level ECS-layer summary that adds `world_tick` and `world_hash`. They are not derived from each other; both are emitted independently. |
| Two different "tick" fields in the persisted state | `aw-save::WorldState.tick` (live) vs. `persistence-ecs::SerializedWorld.world_tick` (hardcoded 0) | Live vs. dormant | The bundle-level `WorldState.tick` is set from the `world_tick` parameter passed into `save_game`. The ECS-level `SerializedWorld.world_tick` is hardcoded to 0 (`persistence-ecs/lib.rs:361`) with TODO comment. The two should arguably equal each other but currently don't. |
| `astraweave-stress-test` declares `astraweave-persistence-ecs` but doesn't use it | `astraweave-stress-test/Cargo.toml:21` vs. (no source-file `use` in that crate) | Unused dep | Same pattern as that crate's `astraweave-net-ecs` declaration (see `docs/architecture/net_ecs.md`). |
| `astraweave-memory` declared as a dep of `astraweave-persistence-ecs` but not imported | `astraweave-persistence-ecs/Cargo.toml:20` vs. (no `use astraweave_memory` in `astraweave-persistence-ecs/src/`) | Unused dep | The dep was probably added in anticipation of saving `CMemory`/`CPersona` through some `astraweave-memory` helper, but the current `SerializedEntity.memory` field stores `Option<CMemory>` directly via `serde`-derived encoding. |
| `auto_save_system` registered as a system but has empty body | `persistence-ecs/lib.rs:66, 72-75` | Active stub | The Plugin wires it into `"post_simulation"`, but the function body is a comment-only TODO. Running the Plugin produces no autosaves. |
| `replay_system` advances `CReplayState.current_tick` but never applies events | `persistence-ecs/lib.rs:67, 78-104` | Active partial stub | The tick counter advances toward `total_ticks`; nothing reads or applies the `events: Vec<ReplayEvent>` field during replay. |
| `CPersistenceManager::save_game` hardcoded companions/inventory | `persistence-ecs/lib.rs:121, 124-127` | Active stub | All bundles emitted through this API have `companions: vec![]` and `inventory: { credits: 1000, items: vec![] }`. Saving a real game requires bypassing or extending this method. |
| Saved files checked into repo: `persistence/aw-save/slot00_*.awsv`, `slot01_*.awsv`, `slot02_*.awsv`, `slot05_*.awsv` + `index.json` | `persistence/aw-save/` | Test artifacts in source tree | 5 `.awsv` files and `index.json` are present at the crate root, not under a `target/` or `tests/fixtures/` subdirectory. Created 2026-03-13 per filename timestamps. |
| `astraweave-persistence-ecs/savegame.bin` (2 bytes) | crate root | Test artifact in source tree | Generated by a doc-example run that wrote `savegame.bin`. |
| Sibling persistence crate `astraweave-persistence-player` (player profiles) | `crates/astraweave-persistence-player/` | Coexisting but disjoint | A separate, workspace-listed crate (`Cargo.toml:106`) for **player-profile** persistence — settings, stats, unlocks, achievements, progression. Uses **TOML** for human-readability (not `.awsv`); has its own postcard-based binary serialization too. **Has zero dependency relationship with `aw-save` or `astraweave-persistence-ecs`** — workspace grep confirms no `use aw_save\|use astraweave_persistence_ecs` in `crates/astraweave-persistence-player/`. Only mention is a single doc-comment hint at `src/save_slots.rs:62`: "`world_state` - Serialized ECS world (from astraweave-persistence-ecs)". The crate carries its own checked-in artifacts: `save.bin`, `saves/`, `test_saves3/`. Functionally orthogonal to the `aw-save`/`persistence-ecs` ECS-world-save subsystem this trace covers. |

### Naming collisions

- **`SaveMeta` (`aw-save`) vs. `SaveMetadata` (`persistence-ecs`):** Distinct types, different fields. Both are public. Confusing under autocomplete.
- **`WorldState` (`aw-save`)** vs. **engine-level "world state"** terminology used in `astraweave-net` etc.: One is a struct, the other is conceptual.
- **`ReplayEvent` (`persistence-ecs`)** vs. **`ReplayEvent` (`astraweave-net`)**: Both are public structs in different crates with different fields. `persistence-ecs::ReplayEvent` is `{ tick, event_type: String, data: Vec<u8> }`; `astraweave-net::ReplayEvent` is `{ tick, seq, actor_id, intent: PlanIntent, world_hash }`. A consumer importing both must qualify.

### Known cognitive traps

- **Trap:** `auto_save_system` runs every tick of `"post_simulation"` but does nothing.
  - **Why it's confusing:** It's wired into the Plugin's `build` method; the system is registered.
  - **What's actually true:** `persistence-ecs/lib.rs:72-75` is a comment-only TODO function body. The Plugin will tick this no-op forever. Adding `PersistencePlugin::new(...)` to an App produces zero automatic saves.

- **Trap:** `replay_system` advances the replay tick counter but the actual event application is not implemented.
  - **Why it's confusing:** `is_replaying` flips to `false` at `total_ticks`, suggesting forward progress.
  - **What's actually true:** `persistence-ecs/lib.rs:96` is `// TODO: Implement replay event application`. The system counts ticks but applies nothing. A loaded `CReplayState` will reach `total_ticks` without changing the world.

- **Trap:** `calculate_world_hash` covers only 4 of 10 component types.
  - **Why it's confusing:** The doc comment for `serialize_ecs_world` lists 10 components; an agent might assume the hash covers all.
  - **What's actually true:** Only `CPos`, `CHealth`, `CTeam`, `CAmmo` enter the hash (`persistence-ecs/lib.rs:641-660`). The function's own doc at `lib.rs:596` admits: `**TODO**: Add CCooldowns, CAiAgent, CPersona, CMemory for complete coverage`. Two worlds differing only in `CMemory` or `CPersona` produce the same hash.

- **Trap:** `calculate_world_hash` skips entities that lack both `CPos` and `CHealth`.
  - **Why it's confusing:** The function presents itself as "hash of the current ECS world state."
  - **What's actually true:** Entity discovery walks only `Query<CPos>` and `Query<CHealth>` (`persistence-ecs/lib.rs:619-633`). An entity holding only `CTeam` or only `CAmmo` doesn't enter the entity list, so its components don't contribute to the hash.

- **Trap:** `SerializedWorld.world_tick` is always 0.
  - **Why it's confusing:** The struct has a `world_tick: u64` field that looks like it carries the simulation tick.
  - **What's actually true:** `persistence-ecs/lib.rs:361`: `world_tick: 0, // TODO: Get from world state when available`. The actual tick is plumbed through `WorldState.tick` at the `aw-save` layer instead.

- **Trap:** `CPersistenceManager::save_game` emits hardcoded inventory and zero companions.
  - **Why it's confusing:** Looks like a complete save API.
  - **What's actually true:** `persistence-ecs/lib.rs:121, 124-127` hardcodes everything except the ECS blob. Production code wanting real companion/inventory data must bypass this method and build `SaveBundleV2` directly.

- **Trap:** `deserialize_ecs_world` on an empty blob returns `Ok(())` without changing the world.
  - **Why it's confusing:** Looks like a successful load.
  - **What's actually true:** `persistence-ecs/lib.rs:447-450` short-circuits on empty input. The caller needs to verify the world has the expected entities; a successful Result does not imply data was loaded.

- **Trap:** `CLegacyId` references are NOT remapped on load.
  - **Why it's confusing:** The doc comment at `persistence-ecs/lib.rs:386-398` describes entity-ID remapping at length.
  - **What's actually true:** Stage 6 above. The id_map is built, but `CLegacyId` is inserted as-is (`lib.rs:490-493`) with a TODO comment. If `CLegacyId` carries an entity reference, the reference points at a (potentially nonexistent) old entity after load.

- **Trap:** Migration generates a fresh `save_id` Uuid.
  - **Why it's confusing:** Migration "preserves" the save's identity in most other dimensions.
  - **What's actually true:** `aw-save/lib.rs:186`: `save_id: Uuid::new_v4()`. The V1 save had no Uuid; the V2 migration mints one. Callers using `save_id` for deduplication should not expect stability across migration.

- **Trap:** `entity_set` in `serialize_ecs_world` is a `HashSet<Entity>` — iteration order is not deterministic across compiles.
  - **Why it's confusing:** The function's doc claims determinism at `lib.rs:275-277`.
  - **What's actually true:** The claim says "HashSet insertion order (same entities → same order → same blob)." Stdlib's `HashSet` uses a randomized hasher by default — `insertion order` is preserved within a single program run, but across runs with different RNG seeds it can differ. Whether the blob is byte-identical across runs depends on the specific stdlib version and hasher; this is **not the same kind of determinism** as `calculate_world_hash`'s explicit `entity_list.sort_unstable()`. [INFERRED — the doc comment overstates the determinism guarantee; not specifically tested with a reproducer.]

- **Trap:** Test artifacts (`.awsv`, `index.json`, `savegame.bin`) are checked into the repo source tree.
  - **Why it's confusing:** Looks like fixture files used by integration tests.
  - **What's actually true:** None of them are explicitly loaded by name in the test files (`grep -rn "savegame.bin\|slot00_2026" astraweave-persistence-ecs/tests persistence/aw-save/tests` returns no name-based loads). They appear to be incidental write-outputs from prior test runs that were never cleaned up.

---

## 7. Decision Log

### Decision: Two-crate split (`aw-save` for format, `astraweave-persistence-ecs` for ECS adapter)
- **Date:** `aw-save` introduced 2025-09-09 in commit `c0d3b0f11` ("Implement comprehensive save/load system with versioning, atomic I/O, and CLI tools (#59)"). `astraweave-persistence-ecs` introduced 2025-10-01 in commit `08befc6ec` ("phase 6 implementation").
- **Status:** Accepted (both crates active).
- **Context:** The same workspace adopts this two-layer split: a generic on-disk format crate, and an ECS-specific adapter that produces opaque blobs for the format.
- **Decision:** Build the file format independent of the ECS shape. ECS knows how to fill `WorldState.ecs_blob: Vec<u8>`; aw-save knows nothing about the contents.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - `aw-save` is reusable for non-ECS state shapes (replay logs, telemetry, etc.).
  - Schema migration (V1 → V2) happens at the `SaveBundleV2` level — not at the ECS blob level. Changing the ECS component set does not bump `SAVE_SCHEMA_VERSION` automatically.

### Decision: `SAVE_SCHEMA_VERSION = 2`, with explicit V1 migration
- **Date:** 2025-09-09 commit `c0d3b0f11`.
- **Status:** Accepted (`aw-save/lib.rs:31`).
- **Context:** Format has already seen one schema bump (V1 had a single optional companion; V2 has a Vec).
- **Decision:** Bump the schema version with each layout change; provide explicit `into_v2()` migration; preserve old data on a best-effort basis (companion becomes a single-element Vec).
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - `read_any_version` and `migrate_file_to_latest` must be extended for every new schema bump.
  - V1→V2 migration generates a fresh `save_id` (no preserved Uuid in V1).

### Decision: Atomic write via tmp + sync_all + rename
- **Date:** 2025-09-09 commit `c0d3b0f11`.
- **Status:** Accepted (`aw-save/lib.rs:258-288`).
- **Context:** Save corruption on crash is unacceptable for game saves.
- **Decision:** Write to `path.with_extension("tmp")`, `sync_all()` to fsync the file, then `fs::rename(tmp, path)` for an atomic replacement.
- **Alternatives considered:** None reasonable for a crash-safe save.
- **Consequences:**
  - On POSIX and NTFS, the rename is atomic — the old file is intact on crash before rename, and only the new file is visible after.
  - No directory fsync is performed after rename. Some filesystems may not persist the directory entry across a crash immediately after rename.

### Decision: LZ4 compression on top of postcard
- **Date:** 2025-09-09 commit `c0d3b0f11`.
- **Status:** Accepted (`aw-save/lib.rs:9, 261`).
- **Context:** LZ4 is fast and the payload is binary postcard.
- **Decision:** `lz4_flex::compress_prepend_size` wraps the postcard payload. Decompress is `lz4_flex::decompress_size_prepended`.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Only one codec byte is recognized (`CODEC_LZ4 = 1`). Adding zstd would require a codec byte bump and parallel branches in `read_any_version`.
  - LZ4 is fast on the write hot path; tradeoff against compression ratio.

### Decision: CRC32 over the compressed payload (not the raw plaintext)
- **Date:** 2025-09-09 commit `c0d3b0f11`.
- **Status:** Accepted (`aw-save/lib.rs:262-264, 322-328`).
- **Context:** The CRC needs to detect tampered or corrupt bytes that would cause decompression to fail or produce garbage.
- **Decision:** CRC32 hashes the post-LZ4 compressed bytes, not the postcard payload.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - A bit-flip in the on-disk file is detected before decompression — fail fast.
  - A bug in lz4 itself (or a different lz4 version) would still produce the same CRC but a different decompressed blob.

### Decision: 10 supported components in `SerializedEntity`
- **Date:** 2025-10-01 commit `08befc6ec`.
- **Status:** Accepted (`persistence-ecs/lib.rs:185-198`).
- **Context:** AstraWeave's `astraweave-core::ecs_components` exposes 10 components. The serialization layer mirrors them as `Option<C>` fields.
- **Decision:** Hardcode a closed 10-field struct rather than use a runtime component registry.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Adding a new component requires editing this crate (5 sites enumerated in the doc comment at `lib.rs:262-268`).
  - No "unknown component" tolerance — saves are bound to a specific component set version.

### Decision: Entity ID remapping (not preservation) on load
- **Date:** 2025-10-01 commit `08befc6ec`.
- **Status:** Accepted (`persistence-ecs/lib.rs:456-462`).
- **Context:** ECS may have other entities already present, or the spawn order may differ across loads.
- **Decision:** Build a `HashMap<u64, Entity>` mapping old `entity_raw` to fresh spawned `Entity`. Old IDs are NOT preserved.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Cross-entity references (e.g., `CLegacyId` if it stores an entity ID) would need to be remapped — and currently are NOT (see §6 trap).
  - Stable cross-save references are the caller's responsibility.

### Decision: `calculate_world_hash` uses `DefaultHasher` (SipHash-1-3)
- **Date:** 2025-10-01 commit `08befc6ec`.
- **Status:** Accepted (`persistence-ecs/lib.rs:612-616`).
- **Context:** The hash is for integrity / replay validation, not security.
- **Decision:** Use Rust's stdlib `DefaultHasher` (which is currently SipHash-1-3 per the rustc default at the time of `a2474c5b7`).
- **Alternatives considered:** Faster non-cryptographic hashes (xxh64, FNV) and stronger cryptographic ones (BLAKE3); both rejected by the doc comment at `lib.rs:536-539` (`"Cryptographically weak (DO NOT use for security) / Fast for integrity checking"`).
- **Consequences:**
  - The hash output value depends on rustc's stdlib hasher choice. If stdlib swaps DefaultHasher's algorithm in a future Rust version, **identical world state would produce different hashes across compilers** — breaking cross-version save validation.
  - Caller code should not store these hashes long-term without versioning.

### Decision: `#![forbid(unsafe_code)]` on both crates
- **Date:** Initial creation.
- **Status:** Accepted (`aw-save/lib.rs:1`, `persistence-ecs/lib.rs:1`).
- **Context:** Save/load is filesystem I/O with no FFI need.
- **Decision:** No `unsafe` anywhere in either crate.
- **Alternatives considered:** None reasonable.
- **Consequences:** All filesystem and crypto / compression work happens in safe Rust through deps (`fs`, `lz4_flex`, `crc32fast`, `postcard`).

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | File magic is `b"ASVS"` (4 bytes) | Yes | `aw-save/lib.rs:28, 311-313` — read_any_version bails on mismatch. |
| 2 | Current schema is `SAVE_SCHEMA_VERSION = 2` | Yes (compile-time) | `aw-save/lib.rs:31, 268`. Migration covers V1; older or future versions bail. |
| 3 | Codec byte must equal `CODEC_LZ4 = 1` | Yes | `aw-save/lib.rs:29, 330-333` — `read_any_version` bails on unknown codec. |
| 4 | CRC32 is verified before decompression | Yes | `aw-save/lib.rs:322-328` — bail on mismatch. |
| 5 | Save filename is `slot{NN}_{timestamp}_{uuid}.awsv` with timestamp using `-` (not `:`) for Windows compatibility | Yes | `aw-save/lib.rs:57-64`; test `test_windows_safe_timestamp` (`lib.rs:358-402`) asserts no invalid Windows characters. |
| 6 | `load_latest_slot` returns the lexicographically-greatest filename, which (per format) is the most recent save | Yes | `aw-save/lib.rs:82-90` — filter + sort + last. Test coverage in `tests/integration_test.rs`. |
| 7 | Migration `SaveBundleV1::into_v2` populates `companions` from the optional `companion` field | Yes | `aw-save/lib.rs:191`. |
| 8 | `serialize_ecs_world` always produces non-empty output even for empty worlds | Yes | Test `serialize_empty_world` (`persistence-ecs/lib.rs:697-703`) asserts. |
| 9 | `deserialize_ecs_world` on empty blob is `Ok(())` no-op | Yes | `persistence-ecs/lib.rs:447-450`; test `deserialize_empty_blob` (`lib.rs:1039-1045`). |
| 10 | Entity IDs are remapped (NOT preserved) on deserialization | Yes | `persistence-ecs/lib.rs:456-501`; roundtrip tests verify components survive (`lib.rs:715-775`) but entity IDs differ. |
| 11 | `calculate_world_hash` is deterministic for the same world state (via `entity_list.sort_unstable()`) | Yes | `persistence-ecs/lib.rs:634`; test `world_hash_consistency` (`lib.rs:777-795`). |
| 12 | Hash changes when any of CPos / CHealth / CTeam / CAmmo changes | Yes | Tests `world_hash_changes_with_modification`, `world_hash_team_component`, `world_hash_ammo_component`, `world_hash_health_only` (`lib.rs:1116-1209`). |
| 13 | Atomic write uses tmp file + sync_all + rename | Yes | `aw-save/lib.rs:275-286`. |
| 14 | `index.json` is sorted by `(slot, created_at)` after each save | Yes | `aw-save/lib.rs:225`. |
| 15 | Sanitized player_id only contains `[A-Za-z0-9_-]` | Yes | `aw-save/lib.rs:342-352` — all other characters become `_`. |
| 16 | Save bundle's `save_id` is unique per save call (new `Uuid::v4()`) | Yes (probabilistic) | `persistence-ecs/lib.rs:138`, `aw-save/lib.rs:186` (migration). Uuid v4 collision probability is negligible. |

---

## 9. Performance & Resource Profile

### Hot paths

- **`serialize_ecs_world`** — designed to be called per autosave. Doc-comment claims: 0.686 ms @ 1,000 entities, 7× faster than 5 ms target, ~0.7 µs per entity, R²=0.999 linear scaling. Throughput: 1.44 Melem/s. Allocates one `HashSet<Entity>`, one `Vec<SerializedEntity>`, and a postcard `Vec<u8>` per call.
- **`deserialize_ecs_world`** — symmetric load path. Doc claims: 1.504 ms @ 1,000 entities, ~1.5 µs per entity including spawn overhead. Two-pass design: spawn loop + insert loop. Allocates a `HashMap<u64, Entity>` for the id_map.
- **`calculate_world_hash`** — claimed 0.594 ms @ 1,000 entities. Used for integrity checking and (when enabled) per-frame validation. Allocates one `Vec<Entity>` + one `DefaultHasher`.
- **`aw-save::write_awsv`** — synchronous I/O hot path. Postcard encode + lz4 compress + CRC32 + tmp file open/write/sync/rename. Disk-IO-bound, not CPU-bound for typical save sizes. Includes a parent-directory `fs::create_dir_all` call on every save (`aw-save/lib.rs:55`).

### Cold paths

- **`SaveManager::list_saves`** — `read_index(dir)` first, falling back to `scan_dir_for_meta(dir)` which decompresses every `.awsv` file in the directory just to extract metadata. Called once per UI render of the save selection screen, typically.
- **`migrate_file_to_latest`** — invoked once per legacy save on first load post-upgrade.
- **`replay_system`** — runs every tick of `"pre_simulation"` but does no actual work (event apply is TODO).
- **`auto_save_system`** — same: runs every tick of `"post_simulation"` and does nothing.

### Resource ownership

- **`SaveManager.root: PathBuf`** — root directory. Cloned on creation; cheap.
- **`CPersistenceManager.save_manager`** — owned by an ECS entity. Single instance per game (assumed; not enforced).
- **`CPersistenceManager.current_player: String`** — switched via `set_player`. No multi-player-in-one-process branching observed.
- **`<root>/<sanitized(player_id)>/`** — per-player directory; created on first save. `.awsv` files accumulate per slot (timestamps in filename → lexicographic sort gives "latest"); old files are NOT auto-pruned. Verified 2026-05-12: comprehensive `grep -rn "fs::remove\|remove_file\|tokio::fs::remove" persistence/aw-save/src` returns zero matches. The directory will grow indefinitely.
- **`index.json`** — rewritten in full on every save (`aw-save/lib.rs:210-229`). Small for typical save counts.
- **`tempfile::tempdir`** — owned by individual tests; auto-deleted on drop.

---

## 10. Testing & Validation

- **Test counts (across both crates):**
  - `astraweave-persistence-ecs/src/lib.rs` (inline): 28 tests
  - `astraweave-persistence-ecs/tests/save_load_tests.rs`: 11 tests
  - `astraweave-persistence-ecs/tests/corruption_recovery_tests.rs`: 13 tests
  - `astraweave-persistence-ecs/tests/large_world_tests.rs`: 8 tests
  - `astraweave-persistence-ecs/tests/mutation_resistant_comprehensive_tests.rs`: 64 tests
  - `astraweave-persistence-ecs/tests/version_migration_tests.rs`: 11 tests
  - `persistence/aw-save/src/lib.rs` (inline): 1 test (Windows-safe timestamp)
  - `persistence/aw-save/tests/integration_test.rs`: 3 tests
  - `persistence/aw-save/tests/migration_test.rs`: 2 tests
  - `persistence/aw-save/tests/mutation_resistant_comprehensive_tests.rs`: 38 tests
- **Total tests in this subsystem:** **179** (135 in persistence-ecs, 44 in aw-save).
- **Mutation testing:** Dedicated mutation-resistance suites (64 + 38 = 102 tests). Not in a centralized workflow.
- **Benchmarks:**
  - `astraweave-persistence-ecs/benches/persistence_ecs_benchmarks.rs` (453 lines)
  - `astraweave-persistence-ecs/benches/world_serialization_benchmarks.rs` (192 lines)
  - `astraweave-persistence-ecs/benches/persistence_ecs_adversarial.rs` (1019 lines)
  - `persistence/aw-save/benches/save_benchmarks.rs` (Criterion)
- **CI presence:**
  - `aw-save` is in `.github/workflows/integration-tests.yml:142` (`cargo test -p aw-save --tests -- --nocapture`).
  - `aw-save` is also referenced in `.github/workflows/sanitizers.yml`.
  - `astraweave-persistence-ecs` is **not** referenced in any workflow as of `a2474c5b7` (verified via workspace grep across `.github/workflows/*.yml`).
- **Miri / Kani validation:** Not in `miri.yml` or `kani.yml`. Both crates carry `#![forbid(unsafe_code)]`.
- **Manual validation:** The 5 checked-in `.awsv` files at `persistence/aw-save/` plus `index.json` are artifacts of prior test runs. `savegame.bin` (2 bytes) at the persistence-ecs crate root is a similar artifact.

---

## 11. Open Questions / Parked Decisions

- **Why is `astraweave-persistence-ecs` declared by `astraweave-stress-test` but not imported?** Workspace grep returns no `use astraweave_persistence_ecs` in `astraweave-stress-test/src/`. **Investigation (2026-05-12):** The dep was added in commit `08befc6ec` (2025-10-01, "phase 6 implementation") — the **same commit** that created `astraweave-persistence-ecs` itself, **and** the same commit that added the parallel `astraweave-net-ecs` dep to the same crate. Both deps were added on day one and neither has been imported. Is this stale residue, future-positioning, or part of a transitive-dep test? Andrew's call.

- **Why is `astraweave-memory` declared as a dep of `astraweave-persistence-ecs` but never imported?** `Cargo.toml:20` pulls the dep in; `grep -rn "use astraweave_memory" astraweave-persistence-ecs/src/` returns no matches. **Investigation (2026-05-12):** The dep was added in commit `08befc6ec` (2025-10-01, "phase 6 implementation") — same commit that created the crate. The dep has never been imported in any subsequent commit. The crate apparently intended to use `astraweave-memory` for `CMemory`/`CPersona` save plumbing but currently emits those fields directly via `serde`-derived encoding. Andrew's call on whether to remove the dep, wire it through, or leave as pre-positioned scaffolding.

- **`auto_save_system` is registered but has a comment-only TODO body.** Should the system be removed pending implementation, gated behind a `feature = "autosave"` flag, or left as a marker for future work? Andrew's call.

- **`replay_system` advances `current_tick` but doesn't apply events.** The replay loop will run to `total_ticks` without changing the world. Is the missing event application a parked feature, or should the system be removed until implementation lands?

- **`CPersistenceManager::save_game` hardcodes inventory and emits zero companions.** Should this API be extended with full inventory / companion parameters, or are callers expected to bypass it and build `SaveBundleV2` directly? Andrew's call.

- **`calculate_world_hash` covers only 4 of 10 components.** The function's own doc comment lists this as a TODO. Should the missing components (`CCooldowns`, `CAiAgent`, `CPersona`, `CMemory`) be added for save-integrity checking? Note that adding fields would change the hash output, breaking any persisted hashes.

- **`calculate_world_hash` uses `DefaultHasher` (SipHash-1-3 via rustc default).** If stdlib changes the DefaultHasher algorithm in a future Rust release, hashes from old builds will not match new ones. Should the crate pin to a specific hasher (e.g., `siphasher` crate) for cross-version stability?

- **`SerializedWorld.world_tick` is hardcoded to 0.** Should the tick be plumbed from the ECS world (which would require an ECS-side "current tick" accessor that doesn't currently exist), or removed from the struct in favor of the bundle-level `WorldState.tick`?

- **Saved entity references via `CLegacyId` are not remapped through `id_map`.** The comment at `persistence-ecs/lib.rs:491` admits this is parked. Should `CLegacyId`'s entity-reference semantics be defined (and the remap implemented), or is the type currently used only for legacy id storage that doesn't reference other entities?

- **`HashSet<Entity>` iteration determinism claim in the doc comment.** `serialize_ecs_world`'s doc claims byte-identical output for the same world state, but stdlib `HashSet` uses a randomized hasher. Whether this is actually byte-deterministic across program runs depends on stdlib internals not enforced by the code. Should the entity discovery be switched to a sorted `Vec` to provide a real guarantee?

- **Test artifacts (5 `.awsv` files + `index.json`) checked into `persistence/aw-save/`** with 2026-03-13 timestamps. **Investigation (2026-05-12):** Committed in `c9ed24c0c` (2026-03-13, "Add input, materials, and PCG scans; implement save file structure") — the commit title is unrelated to the file content, suggesting incidental check-in. `savegame.bin` (2 bytes) at `astraweave-persistence-ecs/` was committed in `3e51f6521` (2025-12-05, "feat: Introduce extensive documentation, new test suites, and core module files…"). Neither commit was about test fixtures. No `persistence/aw-save/tests/fixtures/` directory exists. Comprehensive `grep -rn "slot00_\|slot01_\|slot02_\|slot05_\|savegame\.bin" persistence/aw-save/tests astraweave-persistence-ecs/tests` returns no name-based loads — confirming the files are not test fixtures, they're inadvertent test-run outputs. Should these be moved to a `tests/fixtures/` directory, added to `.gitignore`, or deleted? Andrew's call.

- **Old `.awsv` files are never auto-pruned.** `SaveManager::save` always writes a new timestamped file; `load_latest_slot` finds the newest. The directory grows unboundedly. Is rotation / pruning a parked feature, or is unbounded growth acceptable for the current scale?

- **No `astraweave-persistence-ecs` CI workflow.** `aw-save` is covered by `integration-tests.yml`, but `astraweave-persistence-ecs` is not in any workflow. Is there an expected workflow that should run `cargo test -p astraweave-persistence-ecs`?

---

## 12. Maintenance Notes

**Update this doc when:**
- A new component is added to `astraweave-core::ecs_components` (§3 vocabulary, §5 file map, §6 component-count row, §7 sixth decision).
- `SAVE_SCHEMA_VERSION` is bumped (§3, §7 second decision, §8 invariant 2).
- `auto_save_system` or `replay_system` body is implemented (§2 stages 7-8, §6 stub rows, §11 questions).
- `calculate_world_hash` gains support for additional components or switches its hash algorithm (§2 stage 7, §6 hash trap, §7 eighth decision, §11 hash questions).
- Hash determinism is hardened (§11 stdlib hasher question).
- `astraweave-persistence-ecs` lands a real production consumer (§4 downstream table, §11 first question).
- A CI workflow is added for `astraweave-persistence-ecs` (§10 CI presence note, §11 CI question).
- File format gains a new codec (§7 fourth decision, §8 invariant 3).
- Old-save pruning is implemented (§9 resource ownership note, §11 pruning question).

**Verification process:**
- `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-persistence-ecs/src/lib.rs persistence/aw-save/src/lib.rs` should match §3 vocabulary surface.
- `cargo tree -p astraweave-persistence-ecs --depth 1` should list `aw-save`, `astraweave-ecs`, `astraweave-core`, `astraweave-memory`, `bincode`, `postcard`, `lz4_flex`, `crc32fast`, `serde`, `serde_json`, `time`, `uuid`, `anyhow`.
- `cargo tree -p aw-save --depth 1` should list `anyhow`, `thiserror`, `serde`, `serde_json`, `postcard`, `lz4_flex`, `crc32fast`, `uuid`, `time`, `hex`.
- `rg 'use astraweave_persistence_ecs|use aw_save' --type rust -g '!*test*' -g '!benches/*'` should match §4 consumers; new production consumers must be added.
- `grep -c '#\[test\]\|#\[tokio::test\]' astraweave-persistence-ecs/src/lib.rs astraweave-persistence-ecs/tests/*.rs persistence/aw-save/src/lib.rs persistence/aw-save/tests/*.rs` should total ≥ 179.
- Stamp the new commit hash and date in the metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. The system is **two layers**: `aw-save` (file format, atomic I/O, migration) and `astraweave-persistence-ecs` (ECS adapter). They communicate via `WorldState.ecs_blob: Vec<u8>` which is opaque to `aw-save`.
2. **`aw-save` is production-grade**: atomic writes, CRC32, LZ4, schema migration, Windows-safe timestamps, 256 slots per player.
3. **`astraweave-persistence-ecs` is partially stub**: `auto_save_system` and `replay_system` are TODO; `CPersistenceManager::save_game` hardcodes inventory and companions; `calculate_world_hash` covers only 4 of 10 components; `SerializedWorld.world_tick` is always 0.
4. **No production consumer**: only the crates' own tests/benches use `astraweave-persistence-ecs`. The `astraweave-stress-test` declared dep is unused.
5. **Entity IDs are remapped on load, not preserved.** Cross-entity references via `CLegacyId` are NOT remapped (insertion is as-is).
6. **The `_temp.rs` / temp-file orphans pattern from `astraweave-net-ecs` does NOT exist here.**
7. **CI**: `aw-save` is in `integration-tests.yml`; `astraweave-persistence-ecs` is not in any workflow.

**Files you'll most likely touch:**
- `astraweave-persistence-ecs/src/lib.rs` — serialization changes (add a component → 5 sites enumerated in the doc comment at `lib.rs:262-268`).
- `persistence/aw-save/src/lib.rs` — file format changes (rare); schema migration (one new `SaveBundleVN` + `into_v(N+1)` per bump).

**Files you should NOT touch without strong reason:**
- `persistence/aw-save/tests/mutation_resistant_comprehensive_tests.rs` (38 tests) — mutation-resistance assertions.
- `astraweave-persistence-ecs/tests/mutation_resistant_comprehensive_tests.rs` (64 tests) — same.
- `persistence/aw-save/slot00_*.awsv` and `index.json` — checked-in test artifacts; deleting may break test reproducibility on first run.

**Common mistakes when changing this system:**
- **Adding a new component to the engine without updating `SerializedEntity`, both passes of `serialize_ecs_world`, `deserialize_ecs_world`, and (optionally) `calculate_world_hash`.** The 5-site list in the doc comment is load-bearing.
- **Bumping `SAVE_SCHEMA_VERSION` without writing `SaveBundleVN::into_v(N+1)`.** Old saves become unreadable.
- **Assuming `auto_save_system` produces saves.** It doesn't — body is empty.
- **Assuming `calculate_world_hash` covers all components.** It covers only 4.
- **Storing `calculate_world_hash` results long-term across rustc upgrades.** Hash algorithm depends on stdlib's DefaultHasher.
- **Assuming `SerializedWorld.world_tick` carries the tick.** It's always 0; check `WorldState.tick` instead.
- **Trusting `deserialize_ecs_world`'s `Ok(())` to mean "data loaded".** On empty blob it's a silent no-op.
- **Relying on cross-load entity-reference stability via `CLegacyId`.** References are not remapped.

---

## Appendix B: Historical context

The `aw-save` crate was created **2025-09-09** in commit `c0d3b0f11` ("Implement comprehensive save/load system with versioning, atomic I/O, and CLI tools (#59)") — three weeks before the ECS-Plugin layer. The architecture was clearly designed top-down: figure out the format first, then build ECS plumbing on top.

`astraweave-persistence-ecs` arrived **2025-10-01** in commit `08befc6ec` ("phase 6 implementation") — the same commit that introduced `astraweave-net-ecs`. The two ECS-Plugin layers landed together, both with similar "production-shape stub" issues (declared-but-unused deps, TODO-body systems, scaffolding ready to wire).

The schema has seen one explicit migration (V1 → V2). The V1 shape held a single `Option<CompanionProfile>`; V2 generalizes to `Vec<CompanionProfile>`. V1→V2 migration mints a fresh `save_id` (Uuid::v4) because V1 had no Uuid field.

The 5 `.awsv` files at `persistence/aw-save/` carry timestamps from 2026-03-13, indicating a test run on that date that produced the artifacts and they were committed (possibly inadvertently) rather than ignored.
