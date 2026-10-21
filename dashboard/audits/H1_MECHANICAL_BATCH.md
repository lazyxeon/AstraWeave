# H.1 Mechanical Lint Batch

**Date:** 2026-07-29

**Campaign:** HEALTH, beat H.1

**Status:** Mechanical batch complete; Comprehensive CI Code Quality remains a named red exception

**Branch:** `health/h1-mechanical-lints`

## 1. Scope, provenance, and command boundary

H.1 was executed in the isolated clone
`D:\AstraWeave\target\codex-h1-019f96e4`, never in the active checkout's
working tree.

- Pre-edit enumeration base: `06e612756f527150f769c7a9641e6d26b435b9b9`.
- Final verification base after the 2026-07-29 refresh and clean rebase:
  `9fefac61e05af3eba1de0738d14d47b3809f7a5b`.
- The 16 intervening upstream commits had no path overlap with H.1's 28 changed
  files.
- Toolchain: `rustc 1.89.0 (29483883e 2025-08-04)` and
  `clippy 0.1.89 (29483883ee 2025-08-04)`.
- No manifest, feature, dependency, workflow, lint-tier, or blanket-allowance
  change was made.

The current `.github/workflows/ci.yml` Code Quality command is:

```bash
cargo clippy --workspace --locked $EXCLUDED_PACKAGES \
  --all-features --all-targets -- -D warnings
```

Contrary to the H.1 prompt's parenthetical description, the current aggregate
command does **not** contain `--no-deps`. The accepted CLIP-1 inventory already
handled the literal workspace command's Windows fail-fast boundary by using this
per-primary-package attribution command:

```powershell
cargo clippy --locked -p <package> `
  --all-features --all-targets --no-deps `
  --message-format short -- -D warnings
```

H.1 retained that reproducible attribution boundary. No workflow edit was
authorized or made. The final section reports both the complete per-package
composition and the literal aggregate command separately.

### Exclusion-zone proof

H.1 omitted `astraweave-render`, `astraweave-terrain`, and `tools/aw_editor`
from all edits and from the pre-edit allowed-package sweep. Upstream changed all
three paths during the session; after rebasing, each H.1 branch tree is
byte-identical to `origin/main`:

| Protected path | `HEAD` and `origin/main` tree |
|---|---|
| `astraweave-render` | `b5f5db7fa2d3bc1243b5a848a89c59d547e70b53` |
| `astraweave-terrain` | `393224a1e7cb10f8ded63105a5a2ca2b7380af42` |
| `tools/aw_editor` | `dc42061878fe74d3ec2e32a80ca10dddbdb33ae6` |

## 2. Re-enumeration before edits

The allowed boundary was 119 packages: 133 workspace members minus the 12
workflow exclusions, `astraweave-render`, and `astraweave-terrain`.
`tools/aw_editor` was already one of the 12 workflow exclusions.

The fresh sequential sweep completed all 119 commands rather than stopping at
the first failure:

- 98 packages exited 0.
- 20 packages emitted 117 unique Rust diagnostics.
- `astraweave-blend` was the twenty-first nonzero package but emitted no Rust
  diagnostic because `tikv-jemalloc-sys` could not execute its configure step
  on Windows.
- The matching 2026-07-24 inventory boundary carried 116 diagnostics. The net
  count drift was therefore **+1**, with composition drift in two packages.
- On the H.1 boundary, the accepted inventory classified 71 findings as
  mechanical and 45 as structural. The fresh unresolved adaptive-boss import
  makes the pre-edit composition **71 mechanical / 46 structural**.

| Package | 2026-07-24 inventory | H.1 pre-edit | Drift |
|---|---:|---:|---|
| `adaptive_boss` | 1 | 2 | +1: new all-feature unresolved `OathboundWardenDirector` import |
| `astraweave-ai` | 38 | 38 | Count stable; documented `fast_executor` finding disappeared and new `E0308` appeared |
| `astraweave-asset` | 2 | 2 | None |
| `astraweave-blend` | 0 Rust diagnostics | 0 Rust diagnostics | Windows build boundary persists |
| `astraweave-core` | 1 | 1 | None |
| `astraweave-dialogue` | 1 | 1 | None |
| `astraweave-ecs` | 8 | 8 | None |
| `astraweave-embeddings` | 17 | 17 | None |
| `astraweave-gameplay` | 2 | 2 | None |
| `astraweave-llm` | 8 | 8 | None |
| `astraweave-memory` | 3 | 3 | None |
| `astraweave-net` | 2 | 2 | None |
| `astraweave-physics` | 1 | 1 | None |
| `astraweave-profiling` | 3 | 3 | None |
| `astraweave-rag` | 3 | 3 | None |
| `astraweave-render` | 40 | Not swept pre-edit | Protected by the H.1 exclusion zone |
| `astraweave-scene` | 4 | 4 | None |
| `astraweave-ui` | 2 | 2 | None |
| `astraweave-weaving` | 4 | 4 | None |
| `hello_companion` | 4 | 4 | None |
| `veilweaver_demo` | 3 | 3 | None |
| `veilweaver_slice_runtime` | 9 | 9 | None |
| **Allowed-boundary total** | **116** | **117** | **+1** |

The AI composition change is not a mechanical cleanup credit. The new
`astraweave-ai/src/goap/adapter.rs:737` type error replaced the previously
enumerated and documented `AIArbiter.fast_executor` finding in the count.

## 3. Mechanical fix ledger

H.1 removed **62** diagnostics from the allowed boundary. Each group was
committed independently after its package checks, full unfiltered test suites,
and post-edit per-package Clippy attribution.

The other nine mechanical candidates crossed the classification tripwire below.
Accordingly, all 71 pre-edit mechanical candidates were either removed (62) or
deliberately reclassified (9); none was silently discarded.

| Group | Commit | Accepted mechanical removals | Post-group composition |
|---|---|---:|---|
| AI / ECS / memory / RAG / embeddings | `d7cc8960ad763e9f9d7cb497b82a22ff4f946f21` | 37 | AI 9, ECS 4, memory 3, RAG 0, embeddings 16 |
| Core / physics / asset / scene / gameplay / dialogue / weaving / UI / net / profiling | `10ca7d5226978b18c5921e3bffcf23f9b15c4507` | 14 | Core 1, physics 1, asset 0, scene 0, gameplay 0, dialogue 0, weaving 0, UI 2, net 1, profiling 3 |
| Adaptive boss / hello companion / Veilweaver demo / slice runtime | `9ac074dbd67a344402fe39981028ff6754e1c710` | 11 | Adaptive boss 1, hello companion 3, Veilweaver demo 3, slice runtime 0 |

The accepted edits are lint-suggestion-scale changes: derived or added
`Default` implementations, `or_default`, needless-borrow and closure removal,
unused import/variable removal, direct slices and constants, range
`contains`, initializer construction, and equivalent expression
simplification. The adaptive-boss import received the same feature gate as its
use; the all-feature unresolved import was left untouched.

### Per-package check and test evidence

Every listed `cargo check -p <package>` exited 0. Test counts sum every Rust
test-harness summary for the unfiltered `cargo test -p <package>` invocation.
Those commands use each package's default feature set, as required by H.1;
the attribution command's `--all-features --all-targets` boundary is why it
still exposes the separately deferred compile rot.
The group gates ran immediately before their commits on `06e6127`; after the
non-overlapping upstream rebase, the final workspace check and complete
121-package Clippy sweep were repeated on `9fefac6`.

| Group | Package | Test result | Ignored | Filtered | Post-edit diagnostics |
|---:|---|---:|---:|---:|---:|
| 1 | `astraweave-ai` | 636 passed, 0 failed | 2 | 0 | 9 |
| 1 | `astraweave-ecs` | 835 passed, 0 failed | 30 | 0 | 4 |
| 1 | `astraweave-memory` | 1,022 passed, 0 failed | 0 | 0 | 3 |
| 1 | `astraweave-rag` | 288 passed, 0 failed | 0 | 0 | 0 |
| 1 | `astraweave-embeddings` | 221 passed, 0 failed | 0 | 0 | 16 |
| 2 | `astraweave-core` | 972 passed, 0 failed | 6 | 0 | 1 |
| 2 | `astraweave-physics` | 1,784 passed, 0 failed | 0 | 0 | 1 |
| 2 | `astraweave-asset` | 442 passed, 0 failed | 0 | 0 | 0 |
| 2 | `astraweave-scene` | 453 passed, 0 failed | 1 | 0 | 0 |
| 2 | `astraweave-gameplay` | 730 passed, 0 failed | 0 | 0 | 0 |
| 2 | `astraweave-dialogue` | 222 passed, 0 failed | 0 | 0 | 0 |
| 2 | `astraweave-weaving` | 820 passed, 0 failed | 3 | 0 | 0 |
| 2 | `astraweave-ui` | 323 passed, **1 failed** | 0 | 0 | 2 after revert |
| 2 | `astraweave-net` | 364 passed, 0 failed | 1 | 0 | 1 |
| 2 | `astraweave-profiling` | 44 passed, 0 failed | 0 | 0 | 3 |
| 3 | `adaptive_boss` | 0 passed, 0 failed | 0 | 0 | 1 |
| 3 | `hello_companion` | 0 passed, 0 failed | 0 | 0 | 3 |
| 3 | `veilweaver_demo` | 2 passed, 0 failed | 0 | 0 | 3 |
| 3 | `veilweaver_slice_runtime` | 821 passed, 0 failed | 0 | 0 | 0 |

The green suites total **9,656 passed, 0 failed, 43 ignored, and 0 filtered
out**. The UI suite is not included in that green aggregate.

`astraweave-ui` failed
`persistence::tests::test_save_load_roundtrip` at
`astraweave-ui/src/persistence.rs:121` because `save_settings` could not write
the platform configuration path: **323 passed, 1 failed, 0 ignored, 0
filtered**. Redirecting `APPDATA` into the isolated evidence directory and
rerunning the complete suite produced the same **323/1/0/0** result because the
Windows known-folder lookup ignored that process override. H.1 did not retry
against the real user configuration directory and did not rewrite the test.
The two UI import removals were reverted byte-for-byte, and the final UI Clippy
rerun again emitted both diagnostics.

## 4. Reclassified to structural — H.R input

The classification tripwire rejected nine nominal mechanical candidates:

| Package | Count | Reason for reclassification |
|---|---:|---|
| `astraweave-memory` | 3 | Replacing `f32::max(...).min(...)` with `clamp` changes NaN behavior: the old expressions select numeric bounds while `clamp` propagates NaN. Public memory priority, relationship strength, and proficiency semantics require an intent decision. |
| `astraweave-ai` | 1 | The unused `closing_braces` collection sits in a test whose comment promises first-versus-last JSON comma validation, while its assertion only proves that some `},` exists. Removing or underscore-renaming it would conceal incomplete test intent. |
| `astraweave-ui` | 2 | The import removals themselves were mechanical, but the required full suite cannot establish a green before/after result because of the non-hermetic configuration-path write. H.1's test gate therefore forbids committing them. |
| `astraweave-llm` | 3 | The unused `DType`, unused `rand::Rng`, and deprecated `gen_range` findings are all in `phi3.rs`, which the H.1 anti-drift rule assigns wholesale to the LLM/API restoration beat. |

Two additional fresh structural items were surfaced, not fixed:

- `adaptive_boss`: all-feature `E0432`, unresolved
  `OathboundWardenDirector`.
- `astraweave-ai`: `E0308` at `goap/adapter.rs:737`, expected `Stance` but
  found `String`.

The pre-existing structural/compile/configuration/test-intent set also remains:
six AI recursion-only parameters and its unused scheduler method; four ECS
test-intent diagnostics; 16 embeddings ORT/Candle/dependency failures; eight
Phi-3 findings; net's unexpected feature configuration; the stale core bench
`Pose`; physics `serde_json`; three profiling test-target findings; three
`hello_companion` `Result::context` failures; and three `veilweaver_demo`
compile/control-flow findings.

## 5. Final verification and remaining composition

Final verification ran after rebasing onto `origin/main`:

- `cargo fmt` for all 13 changed packages with `--check`: exit 0.
- `cargo fmt --all --check`: Windows OS error 206 before formatting because the
  workspace-wide rustfmt argument list is too long. This is the previously
  documented FMT-1 Windows command-length boundary; H.1's complete touched
  package set was checked explicitly.
- `cargo check --workspace --locked`: exit 0 in 3m20s. It emitted one
  pre-existing non-denied `tools/aw_editor` unused-import warning and the
  `nalgebra 0.26.2` future-incompatibility notice.
- `git diff --check`: exit 0.
- Independent changed-line audit: no Critical, High, or Medium defect; all 28
  files in scope; no manifest, workflow, exclusion-zone, blanket-allowance,
  unsafe, production-unwrap, or test-intent rewrite.

The authoritative final attribution swept all 121 primary Code Quality
packages, including the protected render and terrain crates but without editing
them:

- **105 packages exited 0; 15 packages emitted 90 unique Rust
  diagnostics; one additional nonzero package (`astraweave-blend`) emitted
  none.**
- The H.1 allowed boundary contains **55 diagnostics across 13 packages**, down
  from 117 across 20 packages.
- Under H.1's final disposition, all 55 are structural, compile,
  configuration, or test-intent work; the 71 pre-edit mechanical candidates
  were either removed or reclassified explicitly.
- `astraweave-blend` remains a separate zero-Rust-diagnostic Windows build
  boundary.

| Remaining package | Final diagnostics | Disposition |
|---|---:|---|
| `adaptive_boss` | 1 | New all-feature compile failure |
| `astraweave-ai` | 9 | Structural/test-intent plus new type error |
| `astraweave-core` | 1 | Stale all-target bench |
| `astraweave-ecs` | 4 | Test-intent |
| `astraweave-embeddings` | 16 | ORT/Candle/feature compile rot |
| `astraweave-llm` | 8 | STOP-scoped Phi-3 API/configuration/mechanical set |
| `astraweave-memory` | 3 | NaN-semantics intent decision |
| `astraweave-net` | 1 | Unexpected feature configuration |
| `astraweave-physics` | 1 | Missing all-feature `serde_json` |
| `astraweave-profiling` | 3 | Test-target compile/assertion debt |
| `astraweave-ui` | 2 | Blocked by non-hermetic full-suite gate |
| `hello_companion` | 3 | Missing `Result::context` trait availability |
| `veilweaver_demo` | 3 | Unresolved crates/control-flow finding |
| `astraweave-render` | 34 | Protected T-series ownership |
| `astraweave-terrain` | 1 | Protected T-series ownership |
| **Primary-package Rust diagnostics** | **90** | Plus blend's separate build boundary |

The protected-zone movement is upstream work, not H.1 credit: render moved from
40 inventory diagnostics to 34, while terrain gained one `type_complexity`
finding at `spline_types.rs:1363`.

The literal current workflow aggregate command returned shell exit 1 before
reaching Rust diagnostics; its nested `tikv-jemalloc-sys` build script exited
101 because it could not execute the jemalloc `configure` command on Windows.
Its fail-fast output is not substituted for the complete per-package
composition above.

## 6. Expected CI board

**Comprehensive CI Pipeline / Code Quality remains red.** H.1 retired 62
behavior-preserving diagnostics, but the final primary-package set still has
90 Rust diagnostics plus the Windows `astraweave-blend`
build boundary. The remaining findings are exactly the protected render/terrain
composition, the nine H.1 reclassifications, and structural/compile/configuration/test-intent
work assigned to H.R, H.2, or the existing T-series owner.

No workflow, package, feature, target, or lint tier was removed or weakened to
manufacture green. Other named CI exceptions in
`CI_WORKSHOP_2026-07.md` were outside H.1 and remain unchanged by this beat.

## 7. Push ledger

The code commits are:

1. `d7cc8960ad763e9f9d7cb497b82a22ff4f946f21` —
   `fix(ai,ecs,rag,embeddings): clear mechanical clippy findings`
2. `10ca7d5226978b18c5921e3bffcf23f9b15c4507` —
   `fix(asset,scene,gameplay,dialogue,weaving,net): clear mechanical lints`
3. `9ac074dbd67a344402fe39981028ff6754e1c710` —
   `fix(adaptive_boss,hello_companion,slice): clear mechanical lints`

The outcome-note recording commit and the final pull-rebase/push result are
cited in the session handoff: a commit cannot embed its own final object ID or
prove a push that occurs after it is created.
