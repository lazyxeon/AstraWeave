---
schema_version: 1
trace_id: security
title: "Security System (script sandbox, anticheat, signatures, path validation, secrets)"
description: "Security + Secrets — script sandbox, anticheat, signatures, path validation, keyring"
primary_crate: astraweave-security
domain: core
lifecycle_status: in_design
integration_status: test_only
owns: [astraweave-secrets, astraweave-security]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Security System (script sandbox, anticheat, signatures, path validation, secrets)

## Metadata

| Field | Value |
|---|---|
| **System name** | Security System (script sandbox, anticheat, signatures, path/deserialization validation, secrets) |
| **Primary crates** | [`astraweave-security`](../../astraweave-security), [`astraweave-secrets`](../../astraweave-secrets) |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Transitional — one surface (path validation) is wired; the rest is in-design-but-tested / dormant scaffolding |
| **Owner notes** | Crates originate from a 2025 "Phase 1 Week 2" remediation effort (see [`docs/archive/remediation/PHASE1_WEEK2_COMPLETE.md`](../../docs/archive/remediation/PHASE1_WEEK2_COMPLETE.md)). Active follow-up work is owned by the S-series security campaign ([`docs/campaigns/security/S0_FINDINGS_SEED.md`](../../docs/campaigns/security/S0_FINDINGS_SEED.md)) and the hardening plan ([`docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md`](../../docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md)). |

---

## 1. Executive Summary

**What this system does:**
Provides a grab-bag of security primitives: path-traversal validation, size-limited deserialization, a Rhai script execution sandbox, an ECS anti-cheat/telemetry plugin, LLM prompt sanitization, ed25519 signing/SHA-256 hashing helpers (`astraweave-security`), and an OS-keyring-backed secret store with a CLI (`astraweave-secrets`).

**Why it exists:**
It was created to close a set of remediation findings (insecure secret storage, unbounded deserialization, path traversal, unsandboxed scripting) catalogued during a 2025 security/remediation pass.

**Where it primarily lives:**
- [`astraweave-security/src/lib.rs`](../../astraweave-security/src/lib.rs) (955 lines incl. tests) — plugin, sandbox, anti-cheat, LLM validator, crypto helpers
- [`astraweave-security/src/path.rs`](../../astraweave-security/src/path.rs) — `safe_under`, `validate_extension`
- [`astraweave-security/src/deserialization.rs`](../../astraweave-security/src/deserialization.rs) — size-limited JSON/TOML/RON parsing
- [`astraweave-secrets/src/`](../../astraweave-secrets/src) — `SecretManager`, `SecretBackend`, `KeyringBackend`, `SecretValue`, and the `aw_secrets` CLI bin

**Status note — read this first.**
This is **not one cohesive runtime system**. As of `7c29b8182` the only surface with non-test, non-example production callers is **path validation** (`astraweave_security::path::safe_under` / `validate_extension`), used by exactly three production sites: [`tools/aw_editor/src/scene_serialization.rs`](../../tools/aw_editor/src/scene_serialization.rs), [`tools/aw_texture_gen/src/main.rs`](../../tools/aw_texture_gen/src/main.rs), and [`tools/aw_demo_builder/src/main.rs`](../../tools/aw_demo_builder/src/main.rs). **Everything else** — `SecurityPlugin`, `ScriptSandbox`/`execute_script_sandboxed`, `CAntiCheat` + the three ECS systems, `sanitize_llm_prompt`, `generate_signature`/`verify_signature`/`hash_data`, the `parse_*_limited` deserialization limiters, and the entire `astraweave-secrets` public API — has **zero production callers** and lives only behind test suites and benchmarks. See §5 and §6 for the per-surface wired/dormant breakdown; this is the single most important fact in this doc.

---

## 2. Authoritative Pipeline

There is no single pipeline. The system is a set of independent utility surfaces. Two are presented below: the **wired** path-validation surface, and the **dormant** ECS security plugin (documented because its existence is load-bearing context for anyone who finds the `SecurityPlugin` symbol).

### 2A. Path validation (WIRED)

```text
[Editor / tool: user-supplied output path]
    │
    │ safe_under(base, user_path)         path.rs:35
    ▼
[validate_user_path_components]           path.rs:117
    role: reject "..", RootDir, Prefix(_) components BEFORE join
    key data: io::Result<()>  (Err on traversal/absolute)
    │
    │ base.canonicalize() then base.join(user_path)   path.rs:40-48
    ▼
[canonicalize-or-walk-parents]            path.rs:52-99
    role: canonicalize combined; if it doesn't exist, walk up to
          deepest existing parent, canonicalize that, re-append the
          non-existent tail
    key data: canonical PathBuf candidate
    │
    │ target_canonical.starts_with(base_canonical)?   path.rs:65 / 102
    ▼
[escape check]
    role: Err(PermissionDenied) if resolved path is not under base
    │
    ▼
[Ok(PathBuf)]  →  validate_extension(path, &["ron","json","toml"])  path.rs:159
    role: extension allowlist (case-SENSITIVE, last-extension only)
    │
    ▼
[fs::read / fs::write at the validated path]   (caller's responsibility)
```

### 2B. ECS security plugin (DORMANT — built only by tests)

```text
[App::new()]
    │
    │ SecurityPlugin::build(&mut app)       lib.rs:128-179
    ▼
[insert_resource: SecurityConfig, TelemetryData, ScriptSandbox, LLMValidator]
    │ add_system("pre_simulation",  input_validation_system)     lib.rs:175
    │ add_system("post_simulation", telemetry_collection_system) lib.rs:176
    │ add_system("post_simulation", anomaly_detection_system)    lib.rs:177
    ▼
[input_validation_system]                  lib.rs:182-229
    role: per-CAntiCheat entity, call validate_player_input, EMA-blend
          trust_score (0.9 old + 0.1 new), push telemetry on anomaly
    │
    ▼
[telemetry_collection_system]              lib.rs:232-250
    role: trim events to last 1000; println! a summary every ~60s
    │
    ▼
[anomaly_detection_system]                 lib.rs:253-290
    role: tally anomalies + low-trust players; emit Critical event if
          >50% of players are low-trust
```

> **Note:** No production `App` registers `SecurityPlugin`. The only `SecurityPlugin::build` calls are in two tests — [`lib.rs:867`](../../astraweave-security/src/lib.rs) (`test_security_plugin_build`, lib.rs:864-873) and [`lib.rs:905`](../../astraweave-security/src/lib.rs) (`mutation_plugin_build_sets_correct_memory_limit`, lib.rs:899-911). The pipeline above is real code but unreachable at runtime. (Corrected — the prior text said this was the *only* build call; there are two, both in tests.)

### Stage-by-stage detail (path validation, the wired surface)

#### Stage 1: Component validation
**File:** [`path.rs:117-137`](../../astraweave-security/src/path.rs)
**Role:** Pre-join rejection of dangerous components.
**Inputs:** `user_path: &Path`.
**Outputs:** `io::Result<()>` — `PermissionDenied` for any `Component::ParentDir` (`..`), `RootDir`, or `Prefix(_)` (Windows drive prefix / absolute).
**Notes:** This runs *before* canonicalization, so a literal `..` is rejected even if the combined path would resolve safely. `Component::CurDir` (`.`) is allowed (tested at `path.rs:432`).

#### Stage 2: Canonicalize-or-walk
**File:** [`path.rs:40-99`](../../astraweave-security/src/path.rs)
**Role:** Resolve the real on-disk path. `base.canonicalize()` requires base to exist (Err `NotFound` otherwise, `path.rs:40-45`). If the combined target doesn't exist, the loop at `path.rs:59-97` walks up to the deepest existing ancestor, canonicalizes it, verifies it is under base, and re-appends the non-existent tail.
**Notes:** On Windows, canonicalization adds the `\\?\` verbatim prefix; tests compare against `base.canonicalize()` rather than the raw base for this reason (`path.rs:191-193`). If the whole path chain is non-existent up to root, it returns the un-canonicalized `combined` (`path.rs:95`) — having already passed component validation.

#### Stage 3: Escape check
**File:** [`path.rs:65-74`, `path.rs:102-111`](../../astraweave-security/src/path.rs)
**Role:** `starts_with(base_canonical)` gate. Catches symlink escapes *post-canonicalization* (the unix-only test at `path.rs:282-304` documents this resolves a symlink and confirms it lands outside base).

#### Stage 4: Extension allowlist
**File:** [`path.rs:159-173`](../../astraweave-security/src/path.rs)
**Role:** `validate_extension(path, allowed)` — checks only the **last** extension (`file.tar.gz` matches `gz`, `path.rs:342`), is **case-sensitive** (`file.PNG` fails against `png`, `path.rs:334`), and Errs `InvalidInput` when there is no extension.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **`safe_under`** | Path-traversal guard returning a canonical PathBuf under a base dir, or an error. | `path.rs:35` |
| **`SecretValue`** | Newtype over `Vec<u8>` that `zeroize()`s on drop; holds a secret's bytes. | `astraweave-secrets/src/backend.rs:4-29` |
| **`SecretBackend`** | Trait (`get`/`set`/`delete`/`list_keys`) abstracting where secrets live. Impls: `KeyringBackend` (prod), `MockBackend` (test-only). | `backend.rs:31-36` |
| **`SecretManager`** | Front door wrapping an `Arc<dyn SecretBackend>`; `global()` lazily builds a keyring-backed singleton. | `manager.rs:6-35` |
| **`ScriptSandbox`** | Holds an `Arc<Mutex<rhai::Engine>>` + `allowed_functions` map + `ExecutionLimits`. | `lib.rs:60-66` |
| **`ExecutionLimits`** | `max_operations`, `max_memory_bytes`, `timeout_ms` for sandboxed scripts. | `lib.rs:68-74` |
| **`CAntiCheat`** | ECS component: `player_id`, `trust_score` (f32), `last_validation`, `anomaly_flags`. The `C` prefix marks it an ECS component. | `lib.rs:85-92` |
| **`trust_score`** | f32 in [0,1]; multiplicatively decayed per detected anomaly, EMA-blended frame-to-frame. | `lib.rs:293-334` |
| **`anomaly_flags`** | Free-form `Vec<String>` of flag names. Only three are interpreted: `rapid_input`, `impossible_movement`, `memory_tamper`. | `lib.rs:299-326` |
| **`LLMValidator`** | Config struct (banned patterns, allowed domains, max length, content-filter toggle) consumed by `sanitize_llm_prompt`. | `lib.rs:77-83` |
| **`TelemetryData` / `TelemetryEvent`** | In-memory event log resource and per-event record. | `lib.rs:33-58` |
| **`ReadLimiter`** | Private `Read` adapter that errors once a byte budget is exhausted. | `deserialization.rs:14-43` |

### Terms to NOT confuse

- **Two unrelated "signature" surfaces.** `astraweave-security`'s `generate_signature`/`verify_signature` (`lib.rs:405-419`) are **ed25519** helpers (via `ed25519-dalek`) with **zero callers**. The networking session-input signing described as "the signature work" is **HMAC-SHA256** and lives entirely in `aw-net-proto` (`SigningKey` / `input_frame_sig_payload`), not here. See [`docs/architecture/net_ecs.md`](net_ecs.md) and [`docs/audits/net_trio_signature_remediation_findings_2026-06.md`](../../docs/audits/net_trio_signature_remediation_findings_2026-06.md). Do not conflate the two; they share no code.
- **"Secrets" (the crate) vs `SecretValue`.** `astraweave-secrets` is the keyring crate; `SecretValue` is its zeroizing byte-wrapper. Neither is referenced by `astraweave-security`.
- **`allowed_functions` (sandbox) vs `allowed_domains` (LLM).** Different structs, different fields; both are **defined but never consulted** (see §6 traps).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-ecs` | `Plugin` trait, `App::add_system`, `World::insert_resource`, `World::entities_with::<CAntiCheat>` | resources + systems | Only exercised by `SecurityPlugin` (dormant); `lib.rs:16,128-179`. |
| `rhai` | `rhai::Engine`, `Scope`, `Dynamic`, `compile`/`eval_ast_with_scope` | script source + context vars | Sandbox internals; `lib.rs:369-403`. |
| `ed25519-dalek`, `sha2`, `hex`, `rand` | `SigningKey`/`VerifyingKey`, `Sha256` | crypto primitives | `lib.rs:405-427`; no internal or external consumer. |
| `keyring` (v3.6) | `keyring::Entry::new`/`get_password`/`set_password`/`delete_credential` | OS credential store under service `"astraweave.secrets"` | `keyring_backend.rs`. |
| `serde_json` / `toml` / `ron` | `from_reader` / `from_str` | config bytes | Deserialization limiters; `deserialization.rs:45-84`. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `tools/aw_editor` (scene serialization) | `astraweave_security::path::{safe_under, validate_extension}` | validated scene file paths | **WIRED.** [`scene_serialization.rs:4,159-163,191-195`](../../tools/aw_editor/src/scene_serialization.rs). |
| `tools/aw_texture_gen` | `path::safe_under` | validated output path | **WIRED.** [`tools/aw_texture_gen/src/main.rs:2,32`](../../tools/aw_texture_gen/src/main.rs). |
| `tools/aw_demo_builder` | `path::safe_under` | validated output path | **WIRED.** [`tools/aw_demo_builder/src/main.rs:2,78`](../../tools/aw_demo_builder/src/main.rs). |
| `astraweave-scripting` | (Cargo dep only) | — | Declares `astraweave-security.workspace = true` in [`astraweave-scripting/Cargo.toml:18`](../../astraweave-scripting/Cargo.toml) but **no `use astraweave_security` appears in its source** — only a comment "Security: Configure limits" at `astraweave-scripting/src/lib.rs:73` where it sets its *own* rhai limits (`src/lib.rs:74-78`). This is a declared-but-unused dependency. Verified — a workspace `rg 'astraweave[_-]security'` over `astraweave-scripting/` returns only `Cargo.toml:18`; there is no `#[cfg(feature)]`-gated `use astraweave_security` anywhere in its source. |
| `astraweave-secrets` consumers | — | — | **NONE.** No non-test, non-bin `use astraweave_secrets` exists in the workspace (the only `use` is in `aw_secrets.rs`). |

### Bidirectional / Coupled

- None. Each surface is standalone; there is no internal coupling between `astraweave-security` and `astraweave-secrets` (they do not depend on each other).

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`astraweave-security/src/path.rs`](../../astraweave-security/src/path.rs) | Path-traversal + extension validation | **Active (WIRED)** | The only surface with production callers (3 sites). `#![forbid(unsafe_code)]` at crate root. |
| [`astraweave-security/src/lib.rs`](../../astraweave-security/src/lib.rs) | `SecurityPlugin`, `ScriptSandbox`, anti-cheat, `LLMValidator`, crypto | **In-design-but-tested (DORMANT)** | All public symbols here lack production callers. Heavy test coverage (lib tests + 9 external test files). |
| [`astraweave-security/src/deserialization.rs`](../../astraweave-security/src/deserialization.rs) | `parse_{json,toml,ron}_limited`, `ReadLimiter` | **In-design-but-tested (DORMANT)** | Zero production callers; only own crate + 2 test files. |
| [`astraweave-secrets/src/manager.rs`](../../astraweave-secrets/src/manager.rs) | `SecretManager`, lazy `global()` singleton | **In-design-but-tested (DORMANT)** | Used only by the `aw_secrets` bin and tests. |
| [`astraweave-secrets/src/backend.rs`](../../astraweave-secrets/src/backend.rs) | `SecretBackend` trait, `SecretValue`, `MockBackend` (test-cfg) | **In-design-but-tested (DORMANT)** | `MockBackend` is `#[cfg(test)]`. `SecretValue` zeroizes on drop. |
| [`astraweave-secrets/src/keyring_backend.rs`](../../astraweave-secrets/src/keyring_backend.rs) | OS-keyring `SecretBackend` impl | **In-design-but-tested (DORMANT)** | `list_keys` returns empty (no metadata index) — `keyring_backend.rs:34-38`. |
| [`astraweave-secrets/src/bin/aw_secrets.rs`](../../astraweave-secrets/src/bin/aw_secrets.rs) | CLI (`set`/`get`/`delete`/`list`/`init`) | **Active (bin)** | The one runnable entry point. `get` prints the secret verbatim (`aw_secrets.rs:39`); `list` is unimplemented (`:46`). |
| [`astraweave-security/tests/*.rs`](../../astraweave-security/tests) | 9 external test suites (~3,600 LoC) | Active (test) | sandbox, anticheat, llm_validation, unicode_bypass, property, boundary, concurrent_stress, error_message, mutation_resistant. |
| [`astraweave-security/benches/security_adversarial.rs`](../../astraweave-security/benches/security_adversarial.rs) | Adversarial benchmark harness (1,209 LoC) | Active (bench) | criterion, `harness = false`. |
| [`astraweave-secrets/benches/secrets_adversarial.rs`](../../astraweave-secrets/benches/secrets_adversarial.rs) | Adversarial benchmark harness (903 LoC) | Active (bench) | criterion, `harness = false`. |

**LoC observation:** The vast majority of this system's lines are in `*_tests.rs` files and benchmarks, not production paths. Test/bench LoC (~5,700+) dwarfs the ~700 lines of production code (cf. [`gh-pages/crates.md`](../../gh-pages/crates.md) lists astraweave-security at 701). Coverage and mutation numbers are high (`docs/current/MUTATION_TESTING_AUDIT.md` records 92.0% raw / 100% adjusted for security, 56.3% raw / 100% adjusted for secrets) — but per CLAUDE.md Key Lesson 8, **"wired beats tested": passing tests with zero callers is dormant code, not a shipped feature.**

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| ed25519 signing (`generate_signature`/`verify_signature`) | `lib.rs:405-419` | Dormant | Coexists with the *real, wired* HMAC-SHA256 input signing in `aw-net-proto`. Different algorithm, different crate, no shared code. [Disposition is S-series's call, not this doc's.] |
| `astraweave-security::path::safe_under` vs ad-hoc `fs` calls | `path.rs` vs the rest of the workspace | Active (wired) but partial adoption | Only 3 production sites adopt it; most file I/O in the workspace does not route through it. [`tools/aw_editor/PRODUCTION_READINESS_AUDIT.md:438`](../../tools/aw_editor/PRODUCTION_READINESS_AUDIT.md) recommends broader adoption. |
| `KeyringBackend` (prod) vs env-var secret reads | `keyring_backend.rs` vs `astraweave-ai/src/orchestrator.rs:376-412` | Both live | The hardening plan (P1 finding 1) notes secrets are *still* read from env vars in the orchestrator/examples while `SecretManager` sits unused. See [`SECURITY_AUDIT_AND_HARDENING_PLAN.md:40-43`](../../docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md). |

### Naming collisions

- **"signature":** In `astraweave-security`, means an ed25519 `Signature` (`lib.rs:405`). In the net stack (`aw-net-proto`), means an HMAC-SHA256 tag over an input frame. These are unrelated; an agent searching "signature" will hit both.
- **"sandbox":** `ScriptSandbox` here (Rhai) is distinct from any GPU/process sandbox. `astraweave-scripting` has its *own* rhai limit configuration (`astraweave-scripting/src/lib.rs:73`) that does **not** use `ScriptSandbox`.

### Known cognitive traps

- **Trap: `ScriptSandbox.allowed_functions` is defined but never consulted.**
  - **Why it's confusing:** The field name implies an allowlist enforced at execution time.
  - **What's actually true:** It is constructed empty (`lib.rs:145,514,830,852`) and **never read** in `execute_script_sandboxed` (`lib.rs:369-403`), which compiles and evals arbitrary Rhai limited only by `max_operations`/`max_string_size`/timeout. This is a *live* security finding: S0-3 in [`S0_FINDINGS_SEED.md:26-31`](../../docs/campaigns/security/S0_FINDINGS_SEED.md).

- **Trap: `sanitize_llm_prompt` does not sanitize.**
  - **Why it's confusing:** The name and `enable_content_filtering` flag imply prompt-injection defense.
  - **What's actually true:** On a suspicious substring (`hack`/`exploit`/`cheat`/`bypass`) it returns `Ok(format!("SAFE: {}", prompt))` — it **prefixes** the unchanged prompt rather than blocking it (`lib.rs:355-363`). It blocks only exact banned-pattern substrings and over-length prompts. Documented as a no-op in [`SECURITY_AUDIT_AND_HARDENING_PLAN.md:64-67`](../../docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md) (P2 finding 1) and CLAUDE.md does not list it but the hardening plan does.

- **Trap: `KeyringBackend::list_keys` always returns `Ok(vec![])`.**
  - **Why it's confusing:** Callers may assume an empty list means "no secrets stored."
  - **What's actually true:** keyring has no enumeration API; the impl returns empty unconditionally and stores no metadata index (`keyring_backend.rs:34-38`). The `aw_secrets list` subcommand prints "Listing not yet implemented" (`aw_secrets.rs:46`). Flagged in [`SECURITY_AUDIT_AND_HARDENING_PLAN.md:77-79`](../../docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md) (P2 finding 4).

- **Trap: `astraweave-scripting` depends on `astraweave-security` but doesn't use it.**
  - **What's actually true:** Cargo dep present (`astraweave-scripting/Cargo.toml:18`); no `use astraweave_security` in source. Declared-but-unused, in the sense of CLAUDE.md Key Lesson 8.

- **Trap: `aw_secrets get` echoes the secret to stdout.**
  - **What's actually true:** `aw_secrets.rs:39` does `println!("{}", value.as_str()?)` with no redaction/`--show` gate. P1 finding 1 in the hardening plan.

---

## 7. Decision Log

### Decision: Create dedicated `astraweave-secrets` crate with an OS-keyring backend
- **Date:** ~2025 (Phase 1 Week 2 remediation)
- **Status:** Accepted; integration into engine crates is still pending (P1 hardening item).
- **Context:** A remediation pass flagged "Insecure Secret Storage" as a High finding ([`docs/archive/reports/PHASE1_COMPLETE.md:24,39`](../../docs/archive/reports/PHASE1_COMPLETE.md)).
- **Decision:** Build a `SecretBackend` trait with a `keyring`-crate-backed impl (Windows Credential Manager / macOS Keychain / Linux Secret Service), fronted by a `SecretManager` singleton and an `aw_secrets` CLI. ([`docs/archive/remediation/PHASE1_WEEK2_COMPLETE.md`](../../docs/archive/remediation/PHASE1_WEEK2_COMPLETE.md))
- **Alternatives considered:** A hybrid OS-keychain + Vault + encrypted-config design was researched ([`docs/archive/remediation/PHASE1_RESEARCH_SUMMARY.md:48-53`](../../docs/archive/remediation/PHASE1_RESEARCH_SUMMARY.md)): Dev = `keyring-rs`, CI = encrypted files, Prod = Infisical/Vault. The shipped crate implements only the Dev-tier `keyring`-rs backend; the encrypted-file and Vault tiers were deferred. (Recovered — the research summary is the comparison; it does not record a formal ADR weighing the rejected tiers against keyring.)
- **Consequences:** Secrets are stored securely *if used*, but the crate is not yet wired into `astraweave-ai`/`astraweave-llm`; env-var reads coexist (see §6 and SECURITY_AUDIT P1).

### Decision: `SecretValue` zeroizes on drop
- **Date:** ~2025
- **Status:** Accepted.
- **Context:** Secret bytes should not linger in freed heap memory.
- **Decision:** `impl Drop for SecretValue { fn drop(&mut self) { self.0.zeroize(); } }` (`backend.rs:25-29`), depending on the `zeroize` crate.
- **Consequences:** Any path holding a `SecretValue` clears it deterministically on drop. [Reasoning beyond the obvious not recovered from available sources.]

### Decision: Pre-join component validation in `safe_under` (reject `..` before canonicalize)
- **Date:** ~2025
- **Status:** Accepted.
- **Context:** Path-traversal protection must work even for non-existent target paths (where `canonicalize` can't run).
- **Decision:** Validate components first (`validate_user_path_components`, `path.rs:117`), then canonicalize-or-walk-parents, then a final `starts_with(base)` escape check that also catches symlink escapes post-canonicalization.
- **Alternatives considered:** [Reasoning not recovered from available sources] — the doc-comments describe behavior but not rejected alternatives.
- **Consequences:** Correct for non-existent paths and symlinks; extension check is case-sensitive and last-extension-only (an intentional simplification per the tests, but not documented as a deliberate trade-off).

### Decision: `#![forbid(unsafe_code)]` on both crates
- **Date:** ~2025
- **Status:** Accepted.
- **Context:** Security-sensitive crates should carry no unsafe.
- **Decision:** Both `astraweave-security/src/lib.rs:1` and `astraweave-secrets/src/lib.rs:1` declare `#![forbid(unsafe_code)]`.
- **Consequences:** No Miri/Kani obligation for these crates (no unsafe to verify).

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `safe_under` never returns a path outside `base` for inputs whose resolved form escapes (incl. `..`, absolute, symlink-escape). | Yes | Tests: `path.rs:216-243,282-304`; property test in `tests/property_tests.rs`. |
| 2 | `safe_under` rejects any `user_path` containing a `..` component, pre-canonicalize. | Yes | `path.rs:121-126`; tests `path.rs:350-356,438-442`. |
| 3 | `validate_extension` matches only the final extension and is case-sensitive. | Yes | Tests `path.rs:334-347`. |
| 4 | `parse_{json,toml,ron}_limited` reject inputs exceeding `MAX_*_BYTES` (JSON 10 MiB, TOML/RON 5 MiB). | Yes | `deserialization.rs:10-12,46-84`; tests `deserialization.rs:107-170,312-367` (boundary uses `>`, so exactly-max passes). |
| 5 | `SecretValue` is zeroized on drop. | Partial (Drop runs; bytes-cleared is harder to assert) | `backend.rs:25-29`; relies on `zeroize` crate semantics. |
| 6 | ed25519 `verify_signature` returns false for tampered data or wrong key. | Yes | Tests `lib.rs:484-495,816-822`. |
| 7 | `validate_player_input` is `is_valid` iff `trust_score > 0.2` (strict). | Yes | Tests `lib.rs:664-708,917-954`. |
| 8 | Neither crate contains `unsafe`. | Yes | `#![forbid(unsafe_code)]` (compiler-enforced). |
| 9 | `KeyringBackend::list_keys` returns an empty Vec (no enumeration). | Yes | `keyring_backend.rs:34-38`. (This is a limitation, recorded as an invariant so callers don't rely on it.) |

> **Anti-invariant (explicitly NOT true):** `ScriptSandbox` does **not** restrict which Rhai functions a script may call; `sanitize_llm_prompt` does **not** block injection. Do not assume these enforce policy (see §6).

---

## 9. Performance & Resource Profile

Performance is largely irrelevant for the wired surface (path validation runs at file-I/O cadence, not per-frame). The dormant ECS systems would run per-tick if wired:

- **`input_validation_system`** (dormant): O(entities with `CAntiCheat`) per `pre_simulation` tick; collects `entities_with` then re-fetches per entity (`lib.rs:182-229`).
- **`TelemetryData.events`** grows unbounded until trimmed to the last 1000 each `post_simulation` tick (`lib.rs:234-237`). The hardening plan (P1 finding 4) flags this as a DoS surface if ever wired.
- **`safe_under`** performs up to two `canonicalize` syscalls plus a parent-walk loop for non-existent paths; cold path, runs at editor save/load and tool-export time.
- **Resource ownership:** `ScriptSandbox` owns `Arc<Mutex<rhai::Engine>>` — a single shared, lock-serialized engine; `execute_script_sandboxed` spawns a `spawn_blocking` task under a `tokio::time::timeout` (`lib.rs:379-400`).

---

## 10. Testing & Validation

- **Unit tests (in-crate):** Extensive `#[cfg(test)]` modules in `lib.rs` (anti-cheat, sanitization, crypto, sandbox, plugin build), `path.rs`, `deserialization.rs`, and all `astraweave-secrets` modules.
- **Integration tests (external):** 9 files in [`astraweave-security/tests/`](../../astraweave-security/tests) (~3,600 LoC): `sandbox_tests`, `anticheat_tests`, `llm_validation_tests`, `unicode_bypass_tests` (795 LoC), `property_tests`, `boundary_condition_tests`, `concurrent_stress_tests`, `error_message_validation_tests`, `mutation_resistant_comprehensive_tests`. `astraweave-secrets/tests/mutation_resistant_comprehensive_tests.rs` for secrets.
- **Mutation testing:** Covered. `docs/current/MUTATION_TESTING_AUDIT.md` records `astraweave-security` at 92.0% raw / 100% adjusted (93 mutants) and `astraweave-secrets` at 56.3% raw / 100% adjusted. In-code "MUTATION REMEDIATION TESTS" exist (`lib.rs:895-954`, `deserialization.rs:307-367`).
- **Property tests:** `tests/property_tests.rs` (proptest dependency, `Cargo.toml:31`).
- **Benchmarks:** `benches/security_adversarial.rs` (1,209 LoC) and `astraweave-secrets/benches/secrets_adversarial.rs` (903 LoC), both criterion `harness = false`.
- **Miri/Kani:** Not applicable — both crates `#![forbid(unsafe_code)]`.
- **Caveat:** High test/mutation scores measure code that, outside `path.rs`, has **no production caller**. The test suite validates behavior of dormant surfaces.

---

## 11. Open Questions / Parked Decisions

- **Is the dormant surface intended to ship, or is it design scaffolding?** `SecurityPlugin`, `ScriptSandbox`, `CAntiCheat`, the deserialization limiters, and the ed25519 helpers have zero callers. The hardening plan (Phases 2-4) describes *intended* integration, but as of `7c29b8182` none has landed. Resolution belongs to the S-series; until then this is in-design-but-tested code.
- **Should `astraweave-scripting` actually use `astraweave-security`?** The dep is declared but unused; `astraweave-scripting` configures its own rhai limits. Is the declared dep vestigial, or a planned-but-unwired integration? [NEEDS VERIFICATION.] (Verification note: the *fact* of non-use is now confirmed in §4 — no `use astraweave_security`, feature-gated or otherwise, exists in the crate. What remains open is the *intent*: whether the dep is vestigial or a planned integration; that is the parked decision.)
- **Should the three live S-series findings (S0-1 net plaintext, S0-2 LLM log leak, S0-3 sandbox allowlist) gate any use of this crate's sandbox?** S0-3 in particular means `execute_script_sandboxed` provides no API-allowlisting; any future wiring must not assume it does. See [`S0_FINDINGS_SEED.md`](../../docs/campaigns/security/S0_FINDINGS_SEED.md).
- **Does ed25519 `generate_keypair` use a CSPRNG?** It calls `SigningKey::from_bytes(&rand::random())` (`lib.rs:417`). `rand::random()` uses the thread RNG (ChaCha-based). The sibling net-proto finding (P2 finding 3) raised RNG-source concerns for `SessionKey`; whether the same scrutiny applies here is unexamined. [NEEDS VERIFICATION.] (Verification note: the crate depends on `rand = "0.9"` (`Cargo.toml:21` → workspace `Cargo.toml:172`); in rand 0.9 the global `rand::random()`/`rng()` is `ThreadRng`, a CSPRNG seeded from the OS and backed by ChaCha12 — so the *generation* is cryptographically sound. The open part is whether the net-proto RNG-source scrutiny needs to be formally applied here; that judgment is parked.)
- **Path-validation adoption gap:** Only 3 of many file-I/O sites route through `safe_under`. Whether broader adoption is intended (per the editor production-readiness audit recommendation) is a parked decision.

---

## 12. Maintenance Notes

**Update this doc when:**
- Any Active file in §5 changes (especially `path.rs` — the only wired surface).
- A dormant surface becomes wired (e.g. `SecurityPlugin` is registered in a production `App`, or `SecretManager` is integrated into `astraweave-ai`/`astraweave-llm`) — at that point move it from DORMANT to Active and re-run the §4 touchpoint analysis.
- An S-series finding (S0-1/2/3) is fixed — update §6 traps and §8 anti-invariants.
- The `astraweave-scripting` declared-but-unused dep is resolved either way.

**Verification process:**
- Re-run the wired/dormant check: `rg 'safe_under|validate_extension|SecurityPlugin|execute_script_sandboxed|SecretManager|parse_.*_limited' --type rust -g '!*test*' -g '!*bench*' -g '!*example*'`. As of `7c29b8182` this returns only the three path-validation tool sites plus the `aw_secrets` bin.
- Spot-check §2 pipelines against current `path.rs` / `lib.rs`.
- Stamp the new commit hash and date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **Only `path::safe_under` / `validate_extension` are wired.** Everything else is tested-but-unwired (in-design) or dormant scaffolding. Don't assume the sandbox, anti-cheat, secrets, or signature code runs at runtime — grep for callers first.
2. **The sandbox doesn't sandbox functions, and the LLM "sanitizer" doesn't sanitize.** `allowed_functions` is never consulted (S0-3); `sanitize_llm_prompt` prefixes `SAFE:` instead of blocking. Treat these as known-incomplete (hardening plan + S-series).
3. **Two different "signature" systems exist.** ed25519 here (dormant) vs HMAC-SHA256 in `aw-net-proto` (wired). They share no code; don't merge them.

**Files you'll most likely touch:**
- [`astraweave-security/src/path.rs`](../../astraweave-security/src/path.rs) — the live surface.
- Callers: [`tools/aw_editor/src/scene_serialization.rs`](../../tools/aw_editor/src/scene_serialization.rs), [`tools/aw_texture_gen/src/main.rs`](../../tools/aw_texture_gen/src/main.rs), [`tools/aw_demo_builder/src/main.rs`](../../tools/aw_demo_builder/src/main.rs).

**Files you should NOT touch without strong reason:**
- The `*_tests.rs` and `*_adversarial.rs` files — large, mutation-tuned; changes ripple into mutation/coverage audits (`docs/current/MUTATION_TESTING_AUDIT.md`).
- `keyring_backend.rs` — touches the OS credential store; `list_keys` emptiness is a documented limitation, not a bug to "fix" without the metadata-index design (hardening plan P2-4).

**Common mistakes when changing this system:**
- **Assuming the security plugin is active.** It is built only by a test. Wiring it requires an `App` registration that doesn't currently exist.
- **Treating high coverage/mutation scores as "shipped."** Per CLAUDE.md Key Lesson 8, tests without callers = dormant code.
- **Adding a second path-validator or signing surface.** Extend `path.rs` / reuse `aw-net-proto`; do not duplicate (CLAUDE.md "never build a second implementation").

---

## Appendix B: Historical context

Both crates trace to a 2025 remediation push ("Phase 1 Week 2", [`docs/archive/remediation/PHASE1_WEEK2_COMPLETE.md`](../../docs/archive/remediation/PHASE1_WEEK2_COMPLETE.md); [`docs/archive/reports/PHASE1_COMPLETE.md`](../../docs/archive/reports/PHASE1_COMPLETE.md)) that addressed a batch of security findings (insecure secret storage, unbounded deserialization, path traversal, unsandboxed scripting). The crates were built and heavily tested, but engine-wide integration was deferred to later phases. A 2025-12 audit ([`docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md`](../../docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md)) re-surveyed the system and produced a 5-phase hardening plan; three of its code-level findings were re-confirmed live at HEAD and routed to the S-series ([`docs/campaigns/security/S0_FINDINGS_SEED.md`](../../docs/campaigns/security/S0_FINDINGS_SEED.md), 2026-06). The most recent substantial code activity was a workspace-wide mutation-test sweep (commit `de531fd09`) rather than feature integration. The net-stack signature work that resolved the old `sign16` finding happened in `aw-net-proto`, **not** in this crate.
