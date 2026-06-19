# Closed-Vocabulary Lint Spec — Documentation Truth campaign

**What this is.** Two closed vocabularies whose members are, by campaign decision, either *forbidden as a present-tense AstraWeave claim* (Vocab A poison strings) or *forbidden as provenance-free self-praise* (Vocab B superlatives). Unlike open-ended capability prose — which only the inventory sweep can audit — these are **closed sets**, so `git grep` is exhaustive where sampling is lossy (the D.1.A failure: it corrected `CLAUDE_MD_HARDENING:296` but missed the sibling at `:319`, and `COMPREHENSIVE:13` but missed `:1022`). This file is the authoritative spec the **D.3 regression lint** consumes; the evidence list of current occurrences is [`CLOSED_VOCABULARY_OCCURRENCES.md`](CLOSED_VOCABULARY_OCCURRENCES.md).

**Lint scope.** All tracked prose: `*.md` plus `gh-pages/**` (`.md`/`.html`/`_config.yml`), `.zencoder/**/*.md`, `.github/*.md`, per-crate `README.md`. **Excluded** (never linted): `docs/journey/**`, `docs/archive/**` (historical artifacts), `docs/campaigns/doc-truth/**` (this campaign's audit trail legitimately *quotes* the forbidden strings as evidence), `.github/copilot-instructions-old-backup.md`. Source-code comments are out of scope (a separate concern from prose).

**Lint posture (per the framework D.3 plan).** Warn-only for two weeks, then enforce. A match is a *candidate* violation; the keep-vs-act rule below defines the false-positive set the lint must allow (cited competitor figures, literal test params, registry-linked headlines, correctly-dated historical notes).

---

## Vocabulary A — poison strings (retired/fabricated values forbidden as current AstraWeave truth)

Seeded from the `CLAIMS_REGISTRY.md` Retired table + D.0 §4 + D.0.1 §5. Each is a value that is FABRICATED (delete) or STALE/superseded (correct/retire) per `D0_CLAIMS_INVENTORY.md` §1.2 ground truth.

```
# Retired entity-capacity / superseded counts
103,500 | 103k | 610,000 | 610k
4,907                      # fluids tests -> 2,560 (registry: fluids-test-markers)
27,000+                    # workspace tests -> ~39,900 markers (registry: test-markers-total)
3,892 | 6,100+             # editor tests -> 9,427 markers (registry: editor-test-markers)
82+ crates | 128 members | 128 workspace | 126 members | 49 production | 47 crates | 44+ crates | 55 crates | 59 library crates
                           # -> 130 members / ~51 production (registry: workspace-members / production-crates)
# Fabricated competitor multipliers / baselines
10.4× | 10.4x | 2.1-5.2× | 2.1-5.2x | 3-6× | 3-6x
Unity 9,900 | Unreal 20k-50k
# Fluids surfaces that were deleted / never existed
ResearchFluidSystem | UnifiedSolver | DFSPH (as live solver) | IISPH (as live solver) | SPH/FLIP (as the solver)
# Dormant-advertised-as-live
99.96%                     # SpatialHash broadphase -> dormant; real broadphase Rapier DefaultBroadPhase
# Wrong runtime LLM default
Hermes 2 Pro | hermes2pro | qwen3:8b | Qwen3-8B   # (as the LIVE default; runtime default is phi3:medium)
# Stale structural counts
7 stages | 7-stage         # -> 8 stages (incl. SYNC)
6 AI modes | 6 planning modes   # -> 7 modes (feature-gated) (registry: ai-modes)
# Phantom cargo aliases (only editor/editor-release/editor-dev exist)
check-all | build-core | test-all | clippy-all | build-working
# Wrong / nonexistent net security + transport
TLS 1.3 | Ed25519          # (as live in-engine net security; real is HMAC-SHA256, no in-engine TLS)
QUIC | quinn               # (as live transport; real is WebSocket / tokio-tungstenite)
# Fabricated type names (do not exist in code)
EntityView | HealthView | ObjectiveHint | HazardHint | MaterialGraph | ShaderTarget | CompileResult | dev_unsigned_assets
```

## Vocabulary B — provenance-free superlatives (forbidden as AstraWeave self-description)

```
world-class | world class
world's first | first AI-native | first ... engine
industry-leading | industry precedent | sets the industry
exceeds industry | exceeds AAA | exceeds Unity | exceeds Bevy | exceeds Unreal
most validated | most comprehensive | most rigorously
rivals Unity | rivals Unreal | matches Unity | matches UE5 | matches/exceeds
parity with AAA | AAA-grade | AAA-class | AAA standards
no other engine | unique in industry | no competitor has this
competitive with UE5 | competitive with Unity
production-grade | production-ready          # status words — see keep-vs-act rule (NOT auto-delete)
```

---

## Keep-vs-act rule (the false-positive allowlist the lint must honor)

For each occurrence:

1. **Poison string asserting the value as current AstraWeave truth** → ACT per four-action: retired numbers DELETE or retire-to-registry; stale-with-§1.2-arbiter CORRECT (link the registry); aspirational REWRITE.
2. **Superlative describing AstraWeave** → remove the provenance-free adjective *surgically*; keep the factual remainder. "world-class geometry rendering but CPU-bound lighting" → drop "world-class", keep the rest. Deleting a superlative ≠ deleting the sentence.
3. **`production-ready` / `production-grade`** → **KEEP** where the surface is genuinely shipped and has production callers; **REWRITE-to-honest** where dormant / in-design / example-scoped. Not an auto-delete.
4. **KEEP byte-identical** when the occurrence is any of:
   - a **cited competitor figure** with provenance (e.g. the Reddit-sourced Unity DOTS "1 million entities" number; "Fyrox is the production-grade Rust+egui reference" — about a *competitor*, not AstraWeave);
   - a **literal test parameter** (e.g. `676` in `astraweave-ai/tests/planner_tests.rs`);
   - the **registry-homed headline** (`12,700` agents) **provided it links** to `CLAIMS_REGISTRY.md#agents-capacity-60fps` rather than restating a bare comparative;
   - a **correctly-dated historical note** in a campaign/audit doc that scopes itself to a past commit (e.g. F.0 audit findings under a "superseded by F.1" banner);
   - an occurrence inside the **excluded paths** (journey/archive/doc-truth/old-backup).

The union of acted + kept occurrences is the complete closed-vocabulary inventory. The lint flags any *new* occurrence (a string from A or B) that is not in the allowlist of rule 4.

---

## Revision history

| Version | Date | Change |
|---|---|---|
| 0.1 (D.1.B) | 2026-06-13 | Created. Vocab A (poison) + Vocab B (superlative) sets seeded from the registry Retired table + D.0/D.0.1 dispositions; keep-vs-act rule + exclusion scope defined for D.3 lint consumption. Pre-edit occurrence totals: 501 A + 415 B across 202 files (see occurrences doc). |
