# Terrain Asset Quality — Campaign Outcome (Research-pass → E-closeout)

| Field | Value |
|---|---|
| **Campaign** | Terrain Asset Quality (TAQ) |
| **Sub-phases** | Research-pass → A.1.Marker (blocked) → A.0.A (fetcher) → A.1 / A.1.B / A.1.C (acquisition) → A.3 (grassland validate) → [resumption reconciliation chain] → D-core (footprint baseline) → asset-hygiene findings → E (this closeout) |
| **Start** | 2026-05-14 (research-pass, commit `b1223b49f`) |
| **Close** | 2026-06-02 (D-core `5881b777f` + hygiene `ad2ed4e82`; E-closeout this commit) |
| **Campaign plan** | `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md` (research-pass through §13.5 candidates) |
| **Research-pass audit** | `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md` |
| **Final-state verdict** | Deployed Tier 1 set baselined: **ACCEPTABLE** at residency (80 MiB/active pack = 31% of the 256 MB soft texture budget) + **OPTIMIZATION-CANDIDATE** on format (uncompressed RGBA8 = 4× the §2.7 compressed budget, because the BC7/KTX2 cook path is broken). Not blocking. |
| **Status** | Closed. Assets acquired, deployed, footprint-baselined, hygiene recorded. Cleanup queue handed off (§5). Pause stack flagged unblocked (§7). |

This document is the durable record of the Terrain Asset Quality campaign. Like the Unified Camera outcome doc it mirrors (`docs/audits/unified_camera_outcome_2026-06.md`) and the Editor-Engine Render Parity outcome doc that one mirrors, it is **immutable** once written: future campaigns reference it; they do not edit it. The campaign **characterized and baselined** terrain asset quality — it did **not** optimize it (the cook-path 4× VRAM win is deferred) and did **not** perfect it (§5's cleanup queue is the proof of remaining work). The register is accurate-and-complete, not triumphal.

A distinctive note up front: **more than half of this campaign was reconciliation, not execution.** TAQ was paused mid-flight when the Unified Camera campaign preempted it, and resumed across a chain of read-only reconciliation sessions (§3) that repeatedly found the remaining work smaller and differently-shaped than the campaign's own notes claimed. That shape is the campaign's primary methodological contribution (§6, Layer 2).

---

## 1. Campaign overview

### What the campaign set out to do

The research-pass (`b1223b49f`, audit at `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md`) framed TAQ as a content-driven engine campaign: replace Tier 1 runtime terrain material **placeholders** (believed to be 14 of 22 solid-color/low-quality materials per Real-Fix.D observations) with high-quality generic PBR ground textures, close the source-acquisition gap for 10 unbaked materials, and verify the renderer scales acceptably to real-PBR content. Six Andrew-gate decisions (a)–(f) were surfaced; the planned sub-phase shape was **A (acquisition + bake) → B (organization, recommend-skip) → C (content upgrade, biome-batched) → D (performance) → E (closeout)**.

### What it achieved

- **Sub-phase A acquisition through-line complete.** A.0.A extended the `astraweave-assets` PolyHaven provider with ARM-map support (unblocking acquisition); A.1 / A.1.B / A.1.C acquired **9 Tier 1 material PNG triples** from PolyHaven and ambientCG into `assets_src/materials/`. The 10th candidate (`default`) was reclassified as dead-code and removed (Editor Multi-Tool follow-up `24b1b2150`).
- **The founding premise was falsified, honestly.** A.3's Phase-1 audit found the acquired real-PBR Tier 1 PNGs were **already deployed at the runtime root and already consumed** by the live biome material path — grassland was already wired with 5 PNG-native layers; the renderer was already PNG-native. The "replace 22 placeholders" premise dissolved. Sub-phase C (content replacement) was re-cast as **demand-driven** (trigger only if downstream work flags a specific texture as inadequate), not a gated mandatory phase.
- **The deployed set was footprint-baselined and verdicted.** D-core (`5881b777f`) computed the absolute GPU texture-memory footprint analytically (exact, because the upload path normalizes everything to uncompressed 1024² RGBA8) and verdicted it against stated budgets: **acceptable at one-active-pack residency (80 MiB), with a format optimization-candidate** (the broken BC7/KTX2 cook path costs 4× the compressed budget §2.7 assumed).
- **Asset hygiene recorded as durable findings** (`ad2ed4e82`): the 27-PNG runtime-vs-source divergence (GPU-invisible; disk/decode hygiene) and the polyhaven 10-missing-files warn-logged silent-fallback.
- **Two methodology candidates matured** for elevation at this closeout: §7.10 (content-vs-structural-defect distinction) and §7.11 (pre-execution validation: input / tool-maturity / provider-fit / scope-fit — five pillars).

### The sub-phases and their commits

| Sub-phase | Commit | Title | Shape |
|---|---|---|---|
| Research-pass | `b1223b49f` | Analytical campaign launch; asset-state + pipeline + organization characterization; 6 Andrew-gates surfaced | audit |
| A.1.Marker | `eab972aea` | BLOCKED: PolyHaven provider has no ARM-packed fetch + no local ImageMagick; Andrew-gate chose pause+extend-fetcher | escalate-defer-flag |
| A.0.A | `6d26c3c15` | Extend `astraweave-assets` PolyHaven provider with "arm" map type; acquisition unblocked | tool-capability extension |
| A.1 | `f56a76124` | 6-material source acquisition (5 fetched + 1 reused); 3 deferred to A.1.B on stale-slug 404s | acquisition |
| A.1.B | `f5387f20e` | 2 verified-slug materials acquired (ice deferred on provider-fit gap) | acquisition |
| A.1.C | `76b15948d` | Ice acquired from ambientCG (final Tier 1 material) | acquisition (alt provider) |
| A.3 | `d13023ebe` | Grassland validate-only — **premise falsified** (already wired + deployed); headless integration test passes | scope-fit audit |
| D-core | `5881b777f` | Absolute texture-footprint baseline; budget-anchored verdict (acceptable + optimization-candidate) | measurement (analytical) |
| Hygiene | `ad2ed4e82` | Durable asset-hygiene findings (27-PNG divergence + polyhaven missing files) | record-completeness |
| E | (this commit) | Campaign closeout: outcome doc + methodology codification (two layers) + cleanup-queue + pause-stack hand-off | documentation-accuracy + record-completeness |

**A.2 (the bake) has no commit** — it stopped at a Phase-3 gate: `toktx`/`basisu` absent on the dev env, and the parallel `bake-texture` path has a placeholder BC encoder + a DFD sRGB bug. Cook deferred indefinitely; the runtime consumes PNGs directly. This is the root of D-core's format optimization-candidate.

**The reconciliation chain (Sub-phases between A.3 and D-core) committed nothing** — they were read-only analytical sessions delivered in-chat (§3). This is itself characteristic of resuming a paused campaign: the work is establishing ground truth, not changing code.

### Why phase numbers are local

References like "A.5," "D.1/D.2/D.3," "§7.11," "Real-Fix.D," "C.9" reflect parallel campaign bookkeeping, not a single global timeline. TAQ's `A.x`/`D.x` numbering is independent of the Editor Multi-Tool Architecture campaign's `Sub-phase N` and the Unified Camera campaign's `C.x`, even where they share methodology lineage (§6) or files. **A concrete instance, recorded for the methodology (§6, Layer 2):** the Unified Camera outcome doc predicted "Terrain Asset Quality resumes at A.5"; the actual resumption found the campaign parked at **A.3** with the A-through-line already substantially complete. The "A.5" was notes-drift. The immutable camera doc is not edited; this is the corrected record.

---

## 2. The deployed terrain material set, final state

### The live binding path

Terrain material binding is keyed on the **legacy 8-variant `BiomeType`** (Grassland, Desert, Forest, Mountain, Tundra, Swamp, Beach, River), not the richer 19-variant `BiomeId`. The terrain-model reconciliation (§3) established this from code: `lib.rs:435` — `biome_map` (BiomeType) "feeds splat-rule selection"; `BiomeId` is consumed only in climate/parameter blending. The archetype system (`WorldArchetypeId`, `RegionalArchetypeMask`) **wraps** biomes as a climate-envelope overlay (default `None`); it does not replace them and does not touch material binding. Biomes are the live terrain primitive and the live material-binding taxonomy.

### The deployed set

- **9 Tier 1 materials** as PNG triples (`<name>.png` + `_n` + `_mra`) at the runtime root `assets/materials/`, consumed via each biome pack's `materials.toml` → `../<material>.png`.
- **6 of 9 are biome-consumed**, spread across 5 biomes: beach[gravel]; mountain[mountain_rock, snow, gravel]; river[mud, gravel, moss]; swamp[mud, moss]; tundra[snow, ice, gravel, mountain_rock].
- **3 of 9 (cobblestone, metal_rusted, wood_planks) are referenced by no biome pack** — UI-only per the campaign plan's §5.2 C.9; zero terrain VRAM.
- **10 biome material packs** exist on disk (beach, desert, forest, grassland, mountain, river, swamp, terrain, tundra + the `polyhaven` showcase pack). All real `BiomeType` biomes have a pack.

### The upload reality (load-bearing for the footprint)

`MaterialManager::build_arrays` (`material_loader.rs:510-536`) **hardcodes a 1024² array dimension** and Lanczos3-resizes every layer on upload, regardless of source PNG resolution. Three D2Array textures per pack — albedo `Rgba8UnormSrgb`, normal `Rgba8Unorm`, mra `Rgba8Unorm`, all 4 bytes/px — with a full 11-mip chain. The cook-to-BC path is broken (A.2), so the runtime is **uncompressed RGBA8**. This single fact makes the footprint exact (no GPU measurement needed) and renders the source-disk resolution spread (1024²/2048²/4096²) **GPU-invisible** — every layer becomes 1024².

---

## 3. The reconciliation chain (the resumption arc)

TAQ's distinctive shape: between A.3 (2026-05-16, pre-pause) and D-core (2026-06-02, post-pause), the Unified Camera campaign preempted TAQ, and resumption ran through **six read-only reconciliation sessions** (all in-chat, no commits) before any execution. Each corrected a confident reconstruction by reading ground truth. They are the empirical spine of §6 Layer 2.

| Reconciliation | The question | What ground truth found |
|---|---|---|
| **Entry-state** | Where did the campaign actually stop? | Parked at A.3, not "A.5" (notes-drift). A-through-line substantially complete; git + filesystem agree; six Andrew-gates re-assessed; pause stack intact. **Lightweight** (no committed doc). |
| **Terrain-model** | Did archetypes replace biomes (stale-ifying A.3)? | No — archetypes **wrap** biomes (climate-envelope overlay). Biomes remain the live material-binding taxonomy (`lib.rs:435`). A.3 holds. **Lightweight.** |
| **biome_showcase repair** | Fix the demo to run the visual gate? | The demo's GPU render path was **never implemented** (`// Rendering code would go here`, present since 2025-09-14). Not minimally fixable. **Escalated** (not a fix). |
| **Gate-scope** | Is the visual-validation gate TAQ-necessary? | No — A.3's gate is splat-shader-functional + aesthetic (cited verbatim), **cross-campaign** (Terrain Material System / render-pipeline owner). Asset-integrity was already proven headless. TAQ routes around it. **Lightweight.** |
| **D-scope fork** | What is Sub-phase D, really? | D-as-§6.2 was a placeholder→replaced delta whose two endpoints no longer both exist (C demand-driven; real PBR already deployed; no clean placeholder baseline). Delta dissolved → reframed to absolute footprint baseline. **Fork reported.** |
| **D-core** (executed) | Absolute footprint + budget verdict | §4. The reframed measurement, committed `5881b777f`. |

The pattern across all six: **the remaining work was smaller and differently-shaped than the queue note claimed**, and the correct first move was always an audit, never a change.

---

## 4. The footprint / budget contract

D-core's analytical result (exact, given hardcoded 1024² + uncompressed RGBA8 + 11 mips, ×1.333 mip multiplier):

| Unit | Footprint |
|---|---|
| Per layer-slot (1024² RGBA8 + mips) | **5.333 MiB** |
| Per material (albedo + normal + mra) | **16.0 MiB** |
| Standard 5-layer biome pack (3 arrays × 5) | **80.0 MiB** |
| Realistic terrain VRAM (one active pack — singular `material_cache`) | **80 MiB** |
| All-10-packs-resident (worst case) | ~800 MiB |

**Budget stated (Pillar 5 — prove it, don't assert it):**
- **§2.7 (doc):** ~384 MB peak @ 2048² **BC7/BC5-compressed**, 32 slots; 1024² fallback "~96 MB". The model assumes compression.
- **Engine (`gpu_memory.rs` defaults):** Textures **soft 256 MB / hard 512 MB**.

**Verdict against the bar: ACCEPTABLE + OPTIMIZATION-CANDIDATE, not blocking.** One active pack at 80 MiB is 31% of the 256 MB soft budget and 21% of the §2.7 peak. But the uncompressed deployment is **4× the per-material VRAM §2.7's own budget assumed** (16 MiB vs 4 MiB compressed) — the single highest-value terrain-texture optimization is fixing the broken BC7/KTX2 cook path (→ 80→~20 MiB/pack), deferred per D.3 (D-core verdicts, does not optimize). Forward risk, not current: if multi-biome blending ever holds many packs resident, N×80 MiB scales toward the 512 MB hard limit; the single-active-pack forward path makes this non-blocking today.

**Claim bounds:** this is texture-memory footprint only — **not** frame-time (the editor 145.5ms / ViewportRenderer-drop alert is editor-runtime perf, routed out per §6.3, §5 item 4), and **not** a placeholder delta (dissolved). The 27-PNG divergence is untouched (§5 item 2).

---

## 5. Post-campaign cleanup queue

Five items, each with what it is, its committed-finding reference, its scope-owner, and why it is **not** a TAQ blocker. None is executed by this closeout (Constraint 1 — E practices the scope-ownership discipline it codifies).

### 5.1 Cook-path BC7/KTX2 fix — the real VRAM win

**What:** the `aw_asset_cli` cook path is broken (no `toktx`/`basisu`; placeholder BC encoder writes monochrome 4×4 blocks; DFD always marks sRGB regardless of `ColorSpace::Linear`, `texture_baker.rs:393`). Fixing it would compress terrain textures ~4× (80 → ~20 MiB/pack). **Reference:** D-core `5881b777f` (§4). **Scope-owner:** its own engine/compression-pipeline session — *not* asset-quality (the third scope-ownership catch, §6 Layer 2). **Not a TAQ blocker:** D-core verdicted the uncompressed footprint *acceptable* at current residency; this is an optimization, not a correctness gap.

### 5.2 27-PNG runtime-vs-source divergence — disk hygiene

**What:** all 9 Tier 1 materials' runtime-root PNGs differ by md5 from their `assets_src/` source copies — a processed deployment with inconsistent processing (some downscaled, some same-res re-encodes) of undocumented provenance. **Reference:** hygiene findings `ad2ed4e82` (full md5 table preserved there). **Scope-owner:** a disk-hygiene cleanup session (canonical-set decision + deployment-process documentation). **Not a TAQ blocker:** **GPU-invisible** per D-core (everything normalizes to 1024² on upload), so it is disk/decode hygiene, not a perf problem.

### 5.3 polyhaven 10 missing files — showcase / silent-failure hygiene

**What:** the `polyhaven` showcase pack references 10 texture files absent anywhere on disk; `build_arrays` warn-logs + substitutes neutrals and the load **reports success** (`material_loader.rs:785-788`) — a warn-logged silent fallback. **Reference:** hygiene findings `ad2ed4e82`. **Scope-owner:** showcase-asset hygiene (acquire / prune / retire the pack). **Not a TAQ blocker:** `polyhaven` is a showcase pack, not a live biome; the 8 real `BiomeType` biomes are unaffected.

### 5.4 Editor frame-alert (145.5ms / ViewportRenderer drop) — editor-perf

**What:** the editor emits a frame-time alert and a `Dropping ViewportRenderer GPU resources` event. **Reference:** routed out in D-core's §6.3 reasoning. **Scope-owner:** editor-perf work. **Not a TAQ blocker:** editor-runtime, **asset-content-independent** (observed with real PBR already deployed), the second scope-ownership catch.

### 5.5 Demo keep-vs-condemn — disposition decision

**What:** `biome_showcase`'s GPU render path was never implemented (stub since 2025-09-14; its working mode is CPU generation + headless stats/export); `unified_showcase` panics and carries five `main_*` backup-variant files. **Reference:** biome_showcase-repair reconciliation (§3). **Scope-owner:** an Andrew demo-disposition decision. **Not a TAQ blocker:** parked, no urgency — once the visual gate routed out (§5 implies no harness need), nothing in TAQ depends on these demos.

**Cross-campaign note:** the splat-coherence visual judgment and the `BiomeId`(19)→material seam are owned by the **Terrain Material System** campaign (`pbr_terrain_forward.wgsl` / per-vertex material data), noted in the gate-scope reconciliation. Not in this queue; flagged as TMS-owned.

---

## 6. Methodology codification — two distinct layers

This campaign contributes methodology at two layers, kept deliberately separate. **Layer 1** elevates the campaign's own planned lesson candidates (§7.10, §7.11) per the campaign plan's §13.3/§13.5. **Layer 2** codifies the methodology that emerged from the *act of resuming a paused campaign* — material the original plan could not have contained. Reinforcement counts are stated as accreted, not inflated (a Layer-2 discipline applied to this codification).

### Layer 1 — the campaign's own lesson candidates, elevated

#### §7.10 — content-vs-structural-defect distinction (elevated, as-amended)

**Principle:** content-quality is a distinct defect class from structural-correctness. Structural defects (pipeline routing, attribute drift, pathway divergence) are verified mechanically (does it reach the renderer; do brush modes work). Content-quality defects (is this texture good enough) are inherently subjective and resist mechanical pass/fail.

**Elevated status — honest amendment.** The campaign-plan §13.3/§13.4 operationalized §7.10 through a pre/post **placeholder baseline** (D.1 placeholder → D.2 post-C comparison) and **biome-batched Sub-phase C** for visual coherence. The resumption arc **falsified that scaffolding**: A.3 found no placeholder state to baseline against (real PBR already deployed), the D-scope fork dissolved the delta, and C became demand-driven. The *abstract distinction holds and is elevated*; its *operationalization was superseded* — D-core replaced the delta with an absolute budget-anchored baseline, and the visual-coherence gate was found cross-campaign (§3 gate-scope). Recording the supersession rather than quietly elevating the dissolved version is itself the honesty discipline (and a Layer-2 instance).

#### §7.11 — pre-execution validation beyond input correctness (elevated; five pillars)

**Principle:** pre-execution validation for any pipeline-touching work must verify (a) **input correctness** at the relevant boundary, (b) every **downstream stage's tool-maturity** independently, (c) **provider-fit** at the material/asset/data-class level (not just slug-correctness within an assumed-correct provider), and (d) **scope-fit** — does this work need doing given the current state of the target artifact. Any of (a)–(d) failing nullifies upstream correctness; all four are independently checkable. This generalizes the Editor Multi-Tool Architecture §7.2 (pre-execution actual-code verification) from "verify the design" to "verify the necessity, the design, and the pipeline maturity at each stage."

**Instances (five pillars, each a real sub-phase failure the chain caught):**
1. **Stale-slug** (A.1, `f56a76124`): 3 of 8 code-read PolyHaven slugs 404'd at live fetch → verify slugs against the live catalog at acquisition time.
2. **Live-verified-slug** (A.1.B, `f5387f20e`): 2 of 2 human-browse-verified slugs fetched first-try → the complement; live verification is the safety net.
3. **Provider-fit** (A.1.C, `76b15948d`): PolyHaven's photoscan catalog has no suitable ice texture → check provider-fit per material *class*, not slug-correctness within an assumed provider.
4. **Tool-maturity** (A.2, no commit): all 9 inputs verifiably correct, but the cook stage is non-functional → audit each pipeline stage's maturity independently; an upstream-correct input cannot rescue a broken downstream stage.
5. **Scope-fit** (A.3, `d13023ebe`): the A.1→A.3 chain solved a problem the codebase had already solved → read the current state of the target artifact before authoring the work. **Highest-impact, cheapest check; run it first.**

### Layer 2 — the resumption-arc body (distinct from Layer 1)

These findings are about *resuming a paused campaign* — the original plan could not contain them. Each is a distinct, citable discipline with its instances from this arc (§3).

#### L2.1 — Resumption is archaeology

**Principle:** resuming a paused campaign is mostly establishing what is true *before any code moves*; the repeated payoff is discovering the remaining work is smaller and differently-shaped than the queue note claimed. The correct first sub-phase of a resumption is almost always an audit, not an execution.

**Instances:** the entire §3 chain — six reconciliations, only the last of which (D-core) changed anything; the "resume at A.5" note resolved to "parked at A.3 with the A-through-line already substantially done."

#### L2.2 — The dissolved-premises chain

**Principle:** treat reconstructed/remembered state as a **hypothesis to falsify, never a fact to build on**. A long detour ages every assumption; nearly every resumption session will correct a confident reconstruction by reading ground truth.

**Instances:** five — "A.5" → actually A.3 (notes-drift); biome→archetype supersession worry → false alarm (archetypes wrap biomes); the assumed render harness → absent (`biome_showcase` never had a GPU path); the visual gate → not TAQ's to own; D's placeholder-delta → no second endpoint exists.

#### L2.3 — The three scope-ownership catches (the sharpest finding)

**Principle:** **scope ownership is a question for gates, measurements, and fixes — not just execution.** An asset-quality campaign is repeatedly tempted by adjacent rendering/runtime/pipeline work that *looks* in-scope but routes to another owner. "Is this ours to own?" must be asked of every gate and every measurement, not only of every edit.

**Instances:** three — the visual gate (splat-shader/aesthetic → Terrain Material System campaign); the editor frame-alert (editor-runtime → editor-perf work); the cook-path BC7/KTX2 fix (compression pipeline → its own pipeline session). Each was *recorded and handed off*, none absorbed. *Kin to L2.4 — see kinship note.*

#### L2.4 — Gates inherit scope discipline

**Principle:** a gate is a dependency. A gate whose failures route mostly to *other* owners is mis-scoped regardless of how reasonable it seemed — scope-creep disguised as rigor, subtler than execution scope-creep because a gate *looks* like diligence.

**Instance:** A.3 set itself a full-pipeline visual-validation gate ("does grassland splat blending look coherent") from within an asset-acquisition campaign. The gate-scope reconciliation (§3) found its failures route to the splat shader (TMS), the biome system, or aesthetic re-fetch — only the last is TAQ's, and its asset-integrity content was already proven headless. The gate was routed out. *A specialization of L2.3, applied to gates.*

#### L2.5 — Falsification turned on the campaign's own assumptions

**Principle:** falsification applies to the campaign's *own* working beliefs, not only to external audit findings. Medium-confidence premises — including ones the analyst and the human believed — must be grep/ground-truth-verified, and some will fall.

**Instances:** two — the biome→archetype supersession worry (falsified: a wrap predating A.3, `lib.rs:435`); the "inconsistent runtime resolutions cause inconsistent VRAM" hypothesis carried into D-core (falsified: `build_arrays` normalizes to 1024², so the divergence is GPU-invisible — §4, §5.2).

#### Kinship note

**L2.3 and L2.4 are one principle at two scopes:** scope-ownership asked of all work (L2.3), and its specialization to gates specifically (L2.4) — a gate is just another piece of work that can belong to someone else. **L2.2 and L2.5 are two faces of a single root:** *reconstructed belief is a hypothesis until ground truth confirms it* — L2.2 is that root applied to inherited campaign state, L2.5 applied to the campaign's own in-flight assumptions. And **L2.1 is the operational consequence** of taking that root seriously: if reconstruction is untrustworthy, resumption must begin by re-establishing truth. Drawing these lines is itself the Unified Camera campaign's measurement-matched-to-seam pillar applied to methodology — matching the codification's shape to the relationship's actual structure rather than flattening five findings into a flat list.

**Lineage:** Layer 2 descends from the Unified Camera campaign's Pillar 2 (audit-shaped sub-phase) and Pillar 4 (falsification discipline), specialized to the *resumption* context. It is TAQ's empirical contribution; the consolidated cross-campaign methodology surface (variously named "§7.11" across campaigns — a phase-numbering-is-local collision with TAQ's *own* §7.11 lesson candidate above) remains deferred to a future boundary, as the prior campaigns also deferred it. Materializing it now would mean drawing on multiple immutable outcome docs into a half-populated surface — a bigger, lower-value move than matching the precedent.

---

## 7. Forward chain after E-closeout

### The campaign is closed

After E commits, the Terrain Asset Quality campaign is complete. The deployed Tier 1 set (§2) is acquired, deployed, and footprint-baselined; the budget verdict (§4) is acceptable-with-an-optimization-candidate; the cleanup queue (§5) is the legible hand-off; both methodology layers (§6) are codified. The campaign characterized and baselined terrain asset quality — the cook-path optimization (the real VRAM win) is deferred to its proper owner, by design.

### The pause stack is unblocked

TAQ was the active campaign blocking two paused predecessors. With TAQ closing, both are **unblocked for resumption**:
- **Editor Multi-Tool Architecture** (Sub-phase 4 regression infrastructure / Sub-phase 5 RegionalArchetypePanel ActiveTool / Mediator Removal / Sub-phase 6 closeout).
- **Regional Archetype Variation** (paused 2026-05-03; H-saveload-diagnostic, F.5-overlay-and-gate, F.6–F.8 closeout).

Their pause was verified clean (entry-state reconciliation, §3). Whichever resumes next should open with **its own entry-state reconciliation** and inherit Layer 2: a paused campaign's queue note is a **hypothesis, not a resumption point** (this campaign's "A.5" was the cautionary instance). Neither is started here — E *flags* them unblocked; resuming either is a separate, deliberate decision.

### Cross-campaign hand-offs

The **Terrain Material System** campaign owns the deferred splat-coherence visual judgment and the `BiomeId`(19)→material per-vertex seam. The **cook-path** optimization (§5.1) is an engine/compression-pipeline concern. None blocks TAQ's closure.

---

## Appendix A — Sub-phase commit summary

| Sub-phase | Commit | Date | Files / Δ |
|---|---|---|---|
| Research-pass | `b1223b49f` | 2026-05-14 | 2 files, +1337/−0 |
| A.1.Marker | `eab972aea` | 2026-05-14 | 1 file, +118/−2 |
| A.0.A | `6d26c3c15` | 2026-05-14 | 3 files, +137/−4 |
| A.1 | `f56a76124` | 2026-05-16 | 19 files, +96/−2 |
| A.1.B | `f5387f20e` | 2026-05-16 | 7 files, +44/−6 |
| A.1.C | `76b15948d` | 2026-05-16 | 4 files, +33/−3 |
| A.3 | `d13023ebe` | 2026-05-16 | 1 file, +108/−6 |
| D-core | `5881b777f` | 2026-06-02 | 1 file, +41/−2 |
| Hygiene | `ad2ed4e82` | 2026-06-02 | 1 file, +44/−0 |
| E-closeout | (this commit) | 2026-06-02 | outcome doc + campaign-doc completion |

**Reconciliation sessions** (entry-state, terrain-model, biome_showcase-repair, gate-scope, D-scope-fork) committed nothing — read-only analytical work delivered in-chat (§3). The acquisition sub-phases (A.1/A.1.B/A.1.C) also wrote asset files to `assets_src/materials/` not reflected in the line counts above (the source PNG triples).

*End of outcome document. The Terrain Asset Quality campaign is closed.*
