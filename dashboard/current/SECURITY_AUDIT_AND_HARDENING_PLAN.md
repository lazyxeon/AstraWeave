# SECURITY_AUDIT_AND_HARDENING_PLAN

**Updated:** 2025-12-06  
**Owner:** Security & Systems Engineering  
**Goal:** Eliminate the currently exploitable gaps in networking, secret handling, LLM integration, and sandboxing so the AstraWeave codebase reaches a verifiable security posture before Phase 9.

---

## Scope & Method
- Reviewed networking crates (`astraweave-net`, `net/aw-net-*`), secret tooling, LLM clients, and the `astraweave-security` systems.
- Traced environment-variable usage, CLI ergonomics, and documentation to see how secrets are handled in practice.
- Looked for plaintext endpoints, weak cryptography, unbounded queues, and logging of sensitive material.
- Results are grouped by severity; remediation phases map to concrete engineering work.

---

## Critical Findings (P0)

1. **No authentication or transport security on the in-engine WebSocket server.**  
   - **Status:** Live finding — owned by the security S-series. See `docs/campaigns/security/S0_FINDINGS_SEED.md` **S0-1** (plaintext `TcpListener::bind`, `token != "dev"` gate; evidence re-confirmed at HEAD `astraweave-net/src/lib.rs:535,786`). Scope-and-fix is S.1, not this doc.

2. **Weak "signature" scheme — RESOLVED.**  
   - **Status:** RESOLVED (Net-Trio-Remediation, 2026-06). The `sign16` XOR stub is deleted; HMAC-SHA256 over the full `SessionKey` landed in `aw-net-proto` (`SigningKey` / `input_frame_sig_payload`), the server verifies first, and the default `SignatureFailurePolicy::Kick` closes the connection on a bad tag. No longer a live finding.

3. **LLM clients log full prompts and responses, leaking proprietary data.**  
   - **Status:** Live finding — owned by the security S-series. See `docs/campaigns/security/S0_FINDINGS_SEED.md` **S0-2** (`eprintln!` prompt/response dump; evidence re-confirmed at HEAD `astraweave-llm/src/lib.rs:176-184,215-222`). Scope-and-fix is S.1.

4. **Script sandbox claims policy enforcement but allows arbitrary Rhai APIs.**  
   - **Status:** Live finding — owned by the security S-series. See `docs/campaigns/security/S0_FINDINGS_SEED.md` **S0-3** (`allowed_functions` defined but never consulted; evidence re-confirmed at HEAD `astraweave-security/src/lib.rs:64`). Scope-and-fix is S.1.

5. **HTTP admin surface for `aw-net-server` is wide open.**  
   - **Evidence:** `net/aw-net-server/src/main.rs:155-173` exposes `/healthz` and `/regions` on `0.0.0.0:8789` without authentication or TLS.  
   - **Impact:** Attackers learn deployment regions, probe liveness, and can script DoS traffic against the control plane.  
   - **Fix Direction:** Bind admin endpoints to localhost by default, add mutual TLS or signed tokens for remote access, and integrate rate limiting/observability.

---

## High Findings (P1)

1. **Secrets are still distributed via environment variables and plaintext CLI output.**  
   - **Evidence:** `docs/configuration/environment-variables.md:11-24` recommends storing `LOCAL_LLM_API_KEY`, `OLLAMA_URL`, etc. in env vars; `examples/llm_integration/src/main.rs:43-118` and `astraweave-ai/src/orchestrator.rs:376-412` read them directly; `astraweave-secrets/src/bin/aw_secrets.rs:39` prints retrieved secrets verbatim.  
   - **Impact:** Keys linger in shell history and CI logs. Anyone running `aw_secrets get` on shared consoles leaks the secret immediately.  
   - **Fix Direction:** Route all secret access through `SecretManager`, redact CLI output by default (require `--show` override), add audit logging, and update docs to forbid env-var storage in prod.

2. **LLM transport defaults to unsecured HTTP.**  
   - **Evidence:** `astraweave-ai/src/orchestrator.rs:376` defaults `OLLAMA_URL` to `http://127.0.0.1:11434`; docs encourage `http://` endpoints.  
   - **Impact:** The moment a remote Ollama host is used (LAN or cloud), prompts and tool outputs traverse the network unencrypted, enabling MITM attacks.  
   - **Fix Direction:** Detect non-local URLs and require `https://` (with client certs or token auth), optionally tunnel via mutually-authenticated gRPC/WebSocket.

3. **Observer spoofing by role-switch requests.**  
   - **Evidence:** `astraweave-net/src/lib.rs:706-720` trusts the `name` string to decide whether a connection sees `player`, `companion`, or `enemy` interest sets.  
   - **Impact:** Anyone can subscribe as `enemy` and receive enemy positions, defeating fog-of-war/privacy.  
   - **Fix Direction:** Bind viewer roles to authenticated identities, link them to ECS entities server-side, and drop the client-provided role hint entirely.

4. **Telemetry and replay queues have no bounds or persistence policies.**  
   - **Evidence:** `astraweave-net/src/lib.rs:491` keeps `Vec<ReplayEvent>` in-memory indefinitely; `astraweave-security/src/lib.rs:177-220` stores all telemetry events and only trims after 1,000 entries without eviction for anomalies.  
   - **Impact:** Malicious clients can bloat RAM (DoS) or force loss of legitimate telemetry through log overflow.  
   - **Fix Direction:** Move replay/telemetry to ring buffers with disk-backed snapshots, enforce per-client quotas, and expose metrics so DoS attempts are obvious.

---

## Medium Findings (P2)

1. **Prompt "sanitization" is a no-op.**  
   - **Evidence:** `astraweave-security/src/lib.rs:333-360` merely prefixes `SAFE:` when suspicious text is found; the original payload still executes.  
   - **Impact:** Prompt-injection protection is illusory, giving engineers a false sense of safety.  
   - **Fix Direction:** Implement structural parsing plus allow/deny lists, integrate with the LLM tool guard, and block/alert on disallowed content.

2. **Admin tooling lacks security observability.**  
   - **Evidence:** `net/aw-net-server/src/main.rs:170-212` spawns HTTP without tracing/logging per request, making intrusion detection difficult.  
   - **Fix Direction:** Add structured tracing, per-endpoint metrics, and audit logs for login attempts, configuration changes, and room lifecycle events.

3. **`SessionKey::random` relies on implicit RNG state.**  
   - **Evidence:** `net/aw-net-proto/src/lib.rs:17-25` calls `rand::rng().fill`, inheriting the default RNG; if seeding is compromised, keys can be predicted.  
   - **Fix Direction:** Switch to `rand::rngs::OsRng` or `ring::rand` for guaranteed CSPRNG output and document rotation cadence.

4. **`aw-secrets` lacks inventory and rotation metadata.**  
   - **Evidence:** `astraweave-secrets/src/keyring_backend.rs:31-45` returns an empty list for `list_keys` and stores no metadata, making audits impossible.  
   - **Fix Direction:** Maintain a shadow index (encrypted) of key IDs with timestamps, enforce rotation reminders, and integrate with GitHub Actions for drift detection.

---

## Remediation Plan

### Phase 0 - Immediate Containment (Day 0-2)
- **Disable insecure defaults:** Refuse plaintext `ws://` and `http://` URLs except for explicit `--allow-insecure-localhost` in debug builds.  
- **Remove sensitive logging:** Strip the prompt/response dumps in `astraweave-llm/src/lib.rs` and gate any future dumps behind `ASTRAWEAVE_DEBUG_REDTEAM=1`.  
- **Lock down CLI output:** Change `aw_secrets get` to redact values unless `--raw` is supplied; add confirmation prompts and audit logs.  
- **Bound replay/telemetry buffers:** Add configurable caps with drop strategies and warnings when limits are crossed.

### Phase 1 - Trustworthy Networking Stack (Week 1)
1. **Protocol Hardening**  
   - Implement HMAC-SHA256 signatures (server + client) using the full 32-byte session key.  
   - Replace `session_key_hint` with a mutually-authenticated handshake that delivers the secret over TLS.  
   - Update `aw-net-client` and sample clients to the new protocol; add compatibility gates to `aw-net-proto`.
2. **Access Control**  
   - Introduce signed session tokens (JWT or Ed25519) issued by the matchmaking service; tie viewer roles to claims, not client-provided strings.  
   - Enforce per-connection interest policies on the server side, ignoring client preferences unless authorized.
3. **Transport Security**  
   - Make TLS mandatory for both `astraweave-net` and `aw-net-server`; embed rustls acceptors directly and expose configuration via `aw_net_server.toml`.  
   - Add optional mTLS or token-based auth for admin endpoints.

### Phase 2 - Secrets & Configuration Modernization (Week 2)
- Integrate `astraweave-secrets` into `astraweave-ai`, `astraweave-llm`, and tooling so LLM keys, webhook secrets, and signing keys never leave the OS keyring/secret manager.  
- Create a `secrets.toml` manifest checked into repo (values in vault) that documents each key, owner, rotation window, and fallback path.  
- Extend the CLI/backend with `list_keys`, metadata, and rotation helpers (`aw_secrets rotate <key>`).  
- Update `docs/configuration/environment-variables.md` to demote env vars to local-dev only; add runbooks describing how to fetch secrets securely in CI (e.g., GitHub OIDC -> cloud secret store).

### Phase 3 - LLM & Data Plane Safeguards (Week 3-4)
- **Transport:** Require HTTPS for non-local LLM endpoints and support per-LLM client certs or signed bearer tokens. Add retry/backoff that respects TLS errors.  
- **Policy enforcement:** Wire `sanitize_llm_prompt` into the orchestrator after rewriting it to actually block disallowed content. Couple with `tool_guard` to halt execution on policy hits.  
- **Data minimization:** Introduce policy-based redaction before storing telemetry or emitting structured logs; run canary tests to ensure no prompts/responses leak to stdout.  
- **Testing:** Add integration tests that assert TLS is enforced and that attempts to connect via plaintext fail with actionable errors.

### Phase 4 - Sandbox & Runtime Security (Week 4-5)
- Refactor the Rhai sandbox to register only approved functions, require explicit capabilities per script, and expose deterministic resource accounting (ops, time, memory).  
- Add fuzz tests and adversarial scripts that attempt to call forbidden APIs, touch the filesystem, or starve the scheduler.  
- Expose sandbox metrics through `astraweave-security` telemetry so the runtime can quarantine abusive scripts automatically.

### Phase 5 - Governance & Continuous Monitoring (Week 5+)
- **CI/CD:** Extend GitHub Actions to run `cargo deny`, `cargo audit`, `git-secrets`, and `trufflehog` on every PR; fail builds if secrets or insecure URLs are introduced.  
- **Runtime Metrics:** Emit Prometheus metrics for auth failures, replay drops, TLS handshake errors, and secret rotations; integrate with existing observability dashboards.  
- **Documentation & Training:** Update `.github/copilot-instructions.md`, `SECURITY.md`, and operator runbooks with the new controls.  
- **Validation:** Schedule quarterly tabletop exercises covering credential compromise, replay DoS, and LLM data exfiltration scenarios.

---

## Success Metrics
- 100% of external endpoints (`ws`, `http`) enforce TLS/mTLS by default.  
- HMAC verification failures are observable and <0.1% of total inputs.  
- Secret inventory shows owner + rotation date for every key; no secrets in env vars outside local dev shells.  
- LLM prompts/responses redaction verified via automated log scans before every release.  
- Sandboxed scripts have zero unauthorized API invocations in the continuous fuzz suite.

---

**Next Steps:** Execute Phase 0 items immediately, then track each phase in the Phase 8 master integration planner. Update this document (and `.github/copilot-instructions.md`) after every completed phase to keep the AI-led workflow aligned with the enforced security posture.
