# Proposal: `aw_trace_sync` — deterministic trace → registry/map synchronization

| Field | Value |
|---|---|
| **Status** | **v1 + v1.1 IMPLEMENTED (2026-06-25)** — `tools/aw_trace_sync` + CI `.github/workflows/trace-sync.yml`. v1.1 added map status sync + `runtime_edges` (see notes below). This doc remains the design-of-record; cargo-topology + `map_overlay.json` (§14) remain deferred. |
| **Target version** | Toolkit automation v1.0 (the "rung 2.5" sweet spot) — shipped |
| **Author note** | Design ratified in discussion 2026-06-25, incorporating an external adversarial critique. Supersedes the ad-hoc manual sync performed during the 2026-06-25 trace campaign (commit `00bb1fac3`). |
| **Owner decision** | Land as a proposal first; `runtime_edges` deferred to v1.1. |

## v1 implementation notes (what shipped vs. this design)

- **Shipped:** front-matter contract (schema below) on all 26 traces; `tools/aw_trace_sync` with `--validate-only` / `--list-untraced` / `--write` / `--check`; CLAUDE.md `TRACE-TABLE` marker-block generation; `workspace_map.html` per-crate `trace`-link sync (in-place, byte-stable); the two prose enumerations retired to pointers; CI `trace-sync.yml` `--check` gate; 9 unit tests.
- **Scope refinement (deferred from §7):** v1 syncs the map's per-crate **`trace` link only**, NOT the map's `statusCategory`/`statusEvidence` — those stay curated. Map **status** sync is moved to **v1.1** (with the overlay + `runtime_edges`), because deriving the map's status taxonomy from `lifecycle_status`/`integration_status` is editorial and better done alongside the overlay extraction.
- **`--list-untraced`** excludes `examples/` crates (demos, not subsystems); it reports the genuinely untraced library/subsystem crates.
- **`docs.yml` was intentionally NOT modified:** the `--check` gate guarantees the committed `workspace_map.html` is always in sync, so the existing "copy `workspace_map.html` → Pages" step already publishes a current map. A pre-build `--write` would be redundant.
- **Determinism confirmed:** the Rust serializer (compact `serde_json` with `preserve_order` + `\uXXXX` non-ASCII escaping) reproduces the existing map blob byte-for-byte, so `--check` is clean and future edits produce minimal diffs.

## v1.1 implementation notes (2026-06-25)

- **Map status sync.** Two new optional front-matter fields are now map-owned: the tool maps `(lifecycle_status, integration_status)` → the map's `(status, statusCategory)` pair (`status_pair`) and writes `status`/`statusCategory` + `statusEvidence` (from `summary`) onto the node. Investigation revealed the map's `statusCategory` is a curated **single dimension** (`active`/`in-design`/`partial`), so the mapping emits only those values and **returns `None` for `unknown` inputs** (leaving the node curated). Eight nodes were corrected on first run (e.g. gameplay/scene/terrain/pcg/water → `partial` for mixed integration; security/ui/quests → `dormant` for test/example-only).
- **Status is owned by the *owning* trace, not just `primary_crate`.** A trace drives its primary crate's status only when `primary_crate ∈ owns`. This was a real bug: `terrain.md` and `terrain_materials.md` both declare `primary_crate: astraweave-terrain`, so keying purely by `primary_crate` let the non-owning slice trace clobber the owner. Slice traces (`terrain_materials`, `animation`; `owns: []`) describe a crate owned elsewhere and must not drive it. (Regression test: `owning_trace_drives_status_not_slice_trace`.)
- **Evidence is a *soft* sync.** `statusEvidence` is overwritten only when front-matter provides a `summary`; otherwise the node's curated evidence is left untouched (never wiped). The 14 traces whose primary already had curated evidence were bootstrapped from it (zero-loss), so they round-trip.
- **`runtime_edges`** (optional `[{to, note}]`, from `primary_crate`): the tool **upgrades the matching cargo edge** by setting `runtime: true` (or adds a new edge if none exists), and the map JS gains one class hook (`${e.runtime ? ' runtime' : ''}`) + one `edge.runtime` style (teal triangle) — a one-time hand edit, since edge styling is JS, not data. Conservatively populated with the one verified edge (`astraweave-physics → astraweave-water`, buoyancy consumes `WaterQuery`); more are added as traces assert them.
- **`schema_version` stays `1`** — both fields are optional, backward-compatible additions.
- **Still deferred (§14):** cargo-`metadata`-derived topology and the curated `map_overlay.json` (both for a future *full-regeneration* model; the in-place surgery here doesn't need them). The map's multi-crate secondary-node statuses (e.g. the 12 non-primary crates under `ai_pipeline`) remain curated — per-crate status isn't expressible in per-trace front-matter.

---

## 1. Problem

The fact *"crate X is documented by trace Y, and its lifecycle/integration status is Z"* currently lives in **three** hand-maintained places that drift independently:

1. The trace doc's own metadata (`docs/architecture/<system>.md`).
2. The CLAUDE.md trace table **and** two prose enumerations (CLAUDE.md ~§"Architecture Trace Maintenance", plus the "Where to Look" and "Reference Files" rows).
3. The `workspace-data` JSON embedded in `docs/architecture/workspace_map.html` (per-crate `trace`/`status`/`statusEvidence` fields, 72-node Cytoscape model).

This is a textbook instance of the workspace's own **§7.7 wrapped-component / multiple-representation hazard**: three sources of the same truth, no single owner, silent divergence. The 2026-06-25 campaign added 13 traces and had to update all three surfaces by hand (the map via a one-off `parse → mutate → re-serialize` Python script). That does not scale and is exactly the drift this proposal removes.

## 2. Goals / non-goals

**Goals**
- One **source of truth** for trace↔crate linkage + status: machine-readable front-matter on each trace doc.
- A **deterministic tool** that regenerates the CLAUDE.md table and the map JSON from that front-matter.
- A **CI `--check` gate** that makes drift *impossible to merge* — this is the load-bearing deliverable, not an optional add-on.
- The LLM keeps the **forensic judgment**; the tool keeps the **mechanical propagation**. Agents never edit shared files.

**Non-goals (this version)**
- Re-deriving the map's node/edge **topology** from `cargo metadata` (deferred — see §14).
- Asserted architectural **`runtime_edges`** in the map (deferred to **v1.1** per owner decision — see §14).
- **Externalizing** the map JSON to a fetched `workspace_data.json` (rejected for now — see §8).
- Re-encoding trace prose (open questions, deep evidence) into front-matter (explicitly avoided — see §3).

## 3. Design principles

1. **Judgment vs. mechanics split.** The LLM owns trace content, the lifecycle/integration verdict, the one-line evidence summary. The tool owns table/map regeneration, sorting, schema validation, duplicate/orphan detection, drift checking. Putting "also update CLAUDE.md and the map" into the *trace prompt* is rejected: parallel agents cannot safely co-edit shared files, and LLM surgery on a 178 KB minified JSON blob is error-prone. Propagation must be deterministic.
2. **Front-matter is a lean projection, not a re-encoding.** It contains **only** what the generated artifacts (table + map) consume. Anything that already has a canonical home in the trace body — §11 Open Questions, detailed evidence, decision log — stays there. Re-encoding it in front-matter would manufacture a *new* drift pair, defeating the purpose. (This is §7.7 applied to the toolkit itself.)
3. **A crate node's status is owned by exactly one trace — the one where it is `primary_crate`.** A trace is many-to-crate (`water.md` documents water + render + physics), but render and physics have their *own* traces that own *their* status. Secondary coverage is a cross-reference link only; it must never propagate status onto another trace's primary crate. (See §5.)
4. **The map JSON is edited in place, not externalized.** The tool performs the same safe `parse `<script id="workspace-data">` → mutate → re-serialize → round-trip-validate` operation already proven in the campaign script. (See §8.)
5. **Deterministic output or the gate is worthless.** Stable sort, fixed formatting, no timestamps, normalized paths — otherwise `--check` produces spurious diffs and gets ignored.
6. **Semantic consistency (front-matter ↔ prose) is checked by the verification pass, not the tool.** The tool cannot read prose; the existing `TRACE_VERIFICATION_PROMPT` already reads the whole doc and is the right layer. (See §10.)

## 4. The front-matter contract (schema v1)

YAML front-matter at the very top of each `docs/architecture/<system>.md`.

**Core (required):**
```yaml
---
schema_version: 1                 # front-matter FORMAT version (distinct from doc_version)
trace_id: water                   # unique across all traces; == filename stem
doc_version: "1.1"                # the trace DOCUMENT's own version (existing §Metadata field)
primary_crate: astraweave-water   # this crate's node owns its status/domain/trace-link from THIS trace
domain: physics-world             # map placement: core | ai | rendering | physics-world | gameplay | networking | tools
lifecycle_status: active          # active | in_design | dormant | deprecated | unknown
integration_status: mixed         # wired | partial | example_only | test_only | dormant | mixed | unknown
summary: "WaterQuery/AnalyticWater facade; physics buoyancy is the sole runtime consumer."
last_verified_commit: 7c29b8182
---
```

**Optional (v1):**
```yaml
also_documents:                   # secondary crates this trace covers — CROSS-REFERENCE LINKS ONLY, never status owners
  - astraweave-render
  - astraweave-physics
risk_level: medium                # low | medium | high | critical | unknown  (optional; map already carries edge risk)
```

**Deferred to v1.1 (do NOT implement in v1):**
```yaml
# runtime_edges:                  # asserted architectural edges -> map, rendered as source:trace_frontmatter
#   - { to: astraweave-physics, kind: consumed_by }
```

**Schema notes**
- `schema_version` ≠ `doc_version`: the first versions the front-matter format; the second is the trace's own document version (1.1, 1.8, …). Both required; do not conflate.
- `unknown` is a first-class value on every status enum so agents are not forced into false certainty.
- `integration_status` deliberately distinguishes `example_only` / `test_only` / `partial` / `dormant` — the three-value `wired|mixed|dormant` collapses distinctions the 2026-06-25 traces actually surfaced (e.g. `astraweave-weaving` = `example_only`, `gameplay::water_movement` = `dormant`, parts of `astraweave-security` = `test_only`).
- No `open_questions`, `owners`, or deep `evidence` fields — those live in the trace body (principle 2).

## 5. Crate-node ownership model

The map and table are keyed by **crate**, but front-matter is keyed by **trace**. Resolution rules the tool MUST apply:

- A crate node's `trace`, `lifecycle_status`, `integration_status`, `domain` come from the trace whose **`primary_crate`** equals that crate.
- A crate appearing only in some trace's `also_documents` gets a **secondary cross-link** (e.g. map tooltip "also covered by water.md") but **no status change** — its status comes from its own primary trace, or is `unknown`/untraced if it has none.
- Exactly one trace may name a given crate as `primary_crate` (validated; duplicates are an error).
- Crates with no `primary_crate` trace are **untraced** (reported by `--list-untraced`; rendered `trace: null` in the map, as today).

This prevents the failure where, e.g., `astraweave-physics`'s node inherits `water.md`'s status merely because water documents physics.

## 6. The tool — `tools/aw_trace_sync` (Rust, cargo-native)

Rust chosen over Python: aligns with the repo's cargo-native, zero-human-code ethos; CI already has the toolchain (`Swatinem/rust-cache` is in `docs.yml`); typed enums make schema validation natural; uses the `cargo_metadata` crate directly. Likely deps: `cargo_metadata`, `serde`, `serde_yaml`, `serde_json`, `clap`, `anyhow`/`thiserror`, `similar` (for `--check` diff output).

**Commands:**
```bash
cargo run -p aw_trace_sync -- --check          # CI gate: nonzero exit if --write would change anything; prints the diff
cargo run -p aw_trace_sync -- --write          # regenerate the CLAUDE.md block(s) + map JSON in place
cargo run -p aw_trace_sync -- --validate-only  # schema + reference validation only, no generation/diff
cargo run -p aw_trace_sync -- --list-untraced  # workspace crates with no primary_crate trace (directly answers "what still needs a trace")
```

**Validations (all of `--validate-only`, plus the generation diff for `--check`):**
- Every trace file parses; front-matter is schema-valid; all enums valid.
- `trace_id` unique; `trace_id` == filename stem.
- Every `primary_crate` and `also_documents` entry resolves to a real workspace crate (`cargo metadata`) OR is explicitly marked `external` / `design-only`.
- At most one trace per `primary_crate`.
- Generated CLAUDE.md block, generated map JSON, and front-matter are mutually consistent.
- The curated overlay (§9) references no nonexistent crate/trace/edge/domain; no orphans; valid enums.
- `--list-untraced`: workspace crates absent from all `primary_crate` fields.

**Canonical output rules (mandatory — the gate depends on them):**
- Stable sort: traces/rows by `trace_id`; crate lists alphabetical except `primary_crate` first.
- Fixed JSON serialization (compact, `ensure_ascii`-style escaping to match the existing blob, deterministic key order), trailing newline.
- Deterministic markdown table formatting.
- **No timestamps** in generated content (avoid `updated_at` churn); date/commit live in each trace's own metadata, not the generated registry.
- **No absolute paths / machine-local data**: normalize all `cargo metadata` paths to workspace-relative; drop volatile fields.

## 7. Generated regions (marker-bounded, tool-owned)

The tool only ever rewrites content **between explicit markers**; everything else in these files is hand-authored and untouched.

- **CLAUDE.md trace table** → wrap in `<!-- TRACE-TABLE:START -->` … `<!-- TRACE-TABLE:END -->`.
- **CLAUDE.md prose enumerations** (the two "(terrain, render, physics, …)" lists in the "Where to Look" and "Reference Files" rows) → **retire them**: replace each with a single generated `<!-- TRACE-LIST:START/END -->` block, or collapse to a pointer to the generated table. They are pure drift surface (principle 1) and must not remain hand-maintained.
- **Map** → the `workspace-data` JSON's per-node `trace` / `lifecycle`/`integration` status / `statusEvidence` fields, regenerated in place (§8). Curated graph topology and overlay fields are preserved.

## 8. Map handling — in-place, not externalized

The map's data is the `textContent` of `<script id="workspace-data" type="application/json">` in `workspace_map.html`. The tool parses that element, applies front-matter-derived updates to per-crate node fields, re-serializes compactly, round-trip-validates the JSON, and writes the file back — the exact, proven operation from the campaign's `update_map.py`.

**Externalizing the JSON to a fetched `workspace_data.json` is rejected for v1.** Browsers block `fetch()` from `file://` (CORS), so a fetched model breaks opening `workspace_map.html` directly — a common quick-look workflow. The hosted Pages copy would work, but the local-file regression is not worth it. Revisit only if docs become server-only.

**GitHub Pages requires no tool-specific step:** `.github/workflows/docs.yml` already copies `docs/architecture/workspace_map.html` → the published `/architecture/` page on every push to `main`. Running `--write` (or `--check`) in that workflow before `mdbook build` keeps the published map current even if a contributor forgets locally.

## 9. Curated overlay — `docs/architecture/_meta/map_overlay.json`

Some map content is genuinely editorial and **cannot** be derived from front-matter or cargo: domain labels, the `domains`/`stories`/`reconciliationNotes` narrative blocks, and per-edge `anomaly`/`risk` annotations (e.g. the `terrain → gameplay` reverse-dep flag). Extract these once into `map_overlay.json` so they survive regeneration.

The overlay is **another validated input**, not an unchecked sidecar. The tool fails on: an overlay entry referencing a nonexistent crate/trace/edge/domain; a duplicate single-valued assignment; a stale story/anomaly reference; a malformed enum; an orphaned annotation.

## 10. Verification-pass integration (semantic guard)

Front-matter can still contradict the prose (e.g. `integration_status: wired` while the body says "no runtime consumer"). The tool cannot catch this — but the existing `TRACE_VERIFICATION_PROMPT_TEMPLATE.md` already reads the whole doc. Add one check to it:

> Confirm the front-matter `lifecycle_status` / `integration_status` / `primary_crate` match the body's wired-vs-dormant verdict (§ exec summary, §5/§6, Appendix A). If they disagree, correct the front-matter to match the evidence and note it in the report.

Deterministic surfacing (an optional generated `<!-- TRACE-SUMMARY -->` block under the front-matter, for mdbook visibility) is **optional**, not required — GitHub already renders YAML front-matter as a table, so the canonical status is visible there.

## 11. CI wiring

A new job runs `cargo run -p aw_trace_sync -- --check` (build cached). Trigger paths must be broad enough that no edit to generated output, inputs, or the tool itself can bypass the gate:

```
docs/architecture/**          **/Cargo.toml            CLAUDE.md
docs/architecture/workspace_map.html
docs/architecture/_meta/map_overlay.json
docs/architecture/_meta/trace_schema.*      (the schema definition)
tools/aw_trace_sync/**        .github/workflows/docs.yml
```
Also wire `--write` (or `--check`) into `docs.yml` ahead of `mdbook build`.

## 12. Migration plan

- **The 15 traces from the 2026-06-25 campaign are ~free to migrate**: their lifecycle/integration verdicts were already produced (e.g. `weaving = example_only`, `water = mixed`, `gameplay = mixed` with `water_movement` dormant). Bootstrapping their front-matter is mechanical.
- **The ~13 pre-existing traces** (physics, net, audio, animation, input, ai_pipeline, ecs_math_core_sdk, persistence_ecs, terrain_materials, render, aw_editor, net_ecs) need a light agent pass to emit front-matter from their existing metadata.
- Bootstrap with `--validate-only` until green, then enable `--write`, then turn on the CI `--check`.

## 13. Build order (each step independently committable)

1. `aw_trace_sync` skeleton: schema types, front-matter parsing, `--validate-only` + `--list-untraced` (read-only; safe to land first; immediately useful).
2. Bootstrap front-matter across all ~28 traces (15 near-free, ~13 via agent pass); get `--validate-only` green.
3. `--write` / `--check` for the CLAUDE.md table + prose-enumeration retirement (marker blocks).
4. Map JSON in-place regeneration (port the campaign `update_map.py` logic into the tool) + extract `map_overlay.json`.
5. CI `--check` job + `docs.yml` integration; add the verification-pass consistency check.

Drift becomes impossible to merge at step 5.

## 14. Explicitly deferred

| Item | Deferred to | Rationale |
|---|---|---|
| **`runtime_edges`** (asserted architectural edges, `source: trace_frontmatter`, rendered distinctly from cargo deps) | **v1.1** | Highest-value map enrichment but the most curation; ship the contract + table + node-status sync first. Owner decision 2026-06-25. |
| **Cargo-metadata-derived topology** (regenerate map nodes/edges from `cargo metadata` instead of the hand-curated node list) | v1.1+ | Cargo proves *declared deps*, not runtime wiring — edges would need provenance (`cargo_dependency` vs `runtime_integration`) to avoid implying false architectural truth. Couple this with `runtime_edges`. |
| **Externalized `workspace_data.json`** | Maybe never | `file://` `fetch()` breakage (§8). |

When v1.1 adds `runtime_edges` + cargo topology, the map's edges must carry `source` provenance and render `cargo_dependency` (structural, low-confidence) differently from `runtime_integration` (asserted, high-confidence) — the wired-vs-dormant lesson at the edge level.

## 15. Open questions / parked decisions

- **Where does each crate's `domain` live** — front-matter (per primary trace, as specced) or the overlay? Specced as front-matter for primary crates; untraced crates need a domain from somewhere (overlay default, or cargo-path heuristic). Resolve during step 4.
- **`risk_level`**: keep as optional front-matter, or derive solely from overlay edge risk? Parked; low stakes.
- **Tool home**: `tools/aw_trace_sync` (alongside `aw_build`, `aw_asset_cli`) assumed. Confirm at build time.
- **Schema definition format** for validation: `serde`-typed in Rust (sufficient) vs. a separate `trace_schema.json`. The Rust types are likely enough for v1.

---

## Appendix A — worked example (`water.md` front-matter)

```yaml
---
schema_version: 1
trace_id: water
doc_version: "1.1"
primary_crate: astraweave-water
domain: physics-world
lifecycle_status: active
integration_status: mixed
summary: "WaterQuery/AnalyticWater facade (sole backend post-W.1); physics buoyancy is the only runtime consumer; render surface + weave accents are example-fed."
last_verified_commit: 7c29b8182
also_documents:
  - astraweave-render
  - astraweave-physics
# runtime_edges (v1.1):  - { to: astraweave-physics, kind: consumed_by }
---
```
Tool effect: sets the `astraweave-water` node `trace=water.md`, `lifecycle=active`, `integration=mixed`, `summary=…`; adds "also covered by water.md" cross-links on the `astraweave-render` and `astraweave-physics` nodes **without** changing their status (those come from the render and physics traces); emits the CLAUDE.md table row for water.

## Appendix B — provenance

This design is the refined output of a 2026-06-25 design discussion that hardened an initial "rung 2" recommendation against an external adversarial critique. Key corrections folded in: the CI `--check` gate is the floor (not optional); the richer `integration_status` enum; cargo edges ≠ architectural truth; primary-crate status ownership; lean front-matter (no re-encoded open-questions); in-place (not externalized) map surgery; broadened CI triggers; canonical-output discipline. The manual sync it replaces is recorded at commit `00bb1fac3`.
