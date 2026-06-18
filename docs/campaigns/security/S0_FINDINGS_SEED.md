# S0 — Security Findings Seed (routed from Documentation Truth D.0.1)

**Purpose.** This is a *routing transcription*, not a security investigation. The Documentation Truth campaign's D.0.1 gap sweep read `docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md` and found three of its security findings still **verified-live at HEAD** by direct grep. They are transcribed here verbatim with their evidence pointers so the S-series owns the actual security work. No fix is proposed or applied here; scope-and-fix is S.1's job.

**Provenance.** Sourced from `docs/campaigns/doc-truth/D01_GAP_INVENTORY.md` §6.4 (CODE-FINDINGs). The cited `file:line` locations were re-confirmed to exist at HEAD `9693649d8` on 2026-06-13.

**Out of scope for this seed (hard limit).** Do not investigate further, do not read beyond confirming the cited lines, do not propose or apply any fix. The correction of `SECURITY_AUDIT_AND_HARDENING_PLAN.md`'s own prose (collapsing its now-stale `sign16` finding, pointing its live findings at this seed) happens in D.1.B, not here.

---

## S0-1 — In-engine WebSocket server is plaintext with no auth

- **claim:** No authentication or transport security on the in-engine WebSocket server; it binds a plaintext `TcpListener` and the only gate is a `token != "dev"` string check.
- **file:line:** `astraweave-net/src/lib.rs:535` (`let listener = TcpListener::bind(addr).await?;`), `astraweave-net/src/lib.rs:786` (`if tok != "dev"`)
- **status:** verified-live at HEAD per D.0.1 grep (2026-06-13)
- **disposition:** scope-and-fix in S.1
- **note:** This is the *in-engine* `astraweave-net` server, distinct from the standalone matchmaking trio (`aw-net-{proto,client,server}`), whose HMAC-SHA256 transport signing is real and enforced (Net-Trio-Remediation, 2026-06).

## S0-2 — LLM clients log full prompts and responses (proprietary-data leak)

- **claim:** LLM clients `eprintln!` the full prompt and full response on every call, leaking prompt/plan content to stderr.
- **file:line:** `astraweave-llm/src/lib.rs:176-184` (prompt dump), `astraweave-llm/src/lib.rs:215-222` (response dump)
- **status:** verified-live at HEAD per D.0.1 grep (2026-06-13); the dump banner is labelled `PROMPT SENT TO LOCAL LLM (via Ollama)` and corroborates the phi3:medium runtime default
- **disposition:** scope-and-fix in S.1

## S0-3 — Rhai script sandbox allowlist is defined but never consulted

- **claim:** The script sandbox claims policy enforcement but allows arbitrary Rhai APIs: the `allowed_functions` allowlist is defined and constructed (empty) but never consulted in the executor path.
- **file:line:** `astraweave-security/src/lib.rs:64` (`pub allowed_functions: HashMap<String, String>,` — defined; constructed empty at :145,:514,:830,:852; no consult site found in the executor path)
- **status:** verified-live at HEAD per D.0.1 grep (2026-06-13)
- **disposition:** scope-and-fix in S.1

---

## Not transcribed (resolved / out of scope)

- The `sign16` weak-signature finding in `SECURITY_AUDIT_AND_HARDENING_PLAN.md:24` is **STALE** — `sign16` was deleted and HMAC-SHA256 (`SigningKey` / `input_frame_sig_payload`, verify-first, `SignatureFailurePolicy::Kick`) landed in `aw-net-proto`. It is *not* a live finding and is not routed here; the plan's own prose correction is a D.1.B task.

---

*Seeded by Documentation Truth D.1.A (2026-06-13). Forward owner: the S-series (S.1 scopes and fixes).*
