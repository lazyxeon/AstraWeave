# Net-Trio Signature Remediation — Findings Record

> **Audit artifact for the Net-Trio-Remediation dynamic workflow (June 2026).** This document is the canonical record of the W.0 recon, the §5 adversarial refute pass, the disposition of every confirmed finding, the known limitations, and the deferred-items log. It feeds two future Andrew-routed efforts — the Dormant Surface Disposition campaign and any session-security work — and starts neither.

| Field | Value |
|---|---|
| **Workflow** | Net-Trio-Remediation (dynamic, headless, fully test-verifiable) |
| **Subsystem** | Standalone matchmaking trio: `net/aw-net-proto`, `net/aw-net-client`, `net/aw-net-server` |
| **Pre-workflow HEAD** | `3cdb23239` |
| **Final commit** | `eb9977b88` (+ this doc-only closeout) |
| **Date** | 2026-06-10 |
| **Mission** | Make packet signing real (HMAC-SHA256), verified (constant-time), and enforced (kick-by-default), and build a regression net hard enough that this class of defect cannot recur silently. |
| **Outcome** | Defect fixed and enforced; §5 refute loop CONVERGED; all closure proofs pass. |

---

## 1. The defect (confirmed at W.0)

The trio's packet signature verification failed on **every** packet, and the server's response to failure was a `warn!` log, not a rejection — authentication was 100% broken and 100% silent.

- **Client** signed `InputFrame` with `sign16` (`aw-net-proto`): a `DefaultHasher` XOR-fold producing a 16-byte tag keyed by an 8-byte `session_key_hint` (the first 8 bytes of a per-room `SessionKey`).
- **Server** verified with HMAC-SHA256 over the full 32-byte `SessionKey`, via `mac.verify_slice(&sig)`.
- Two independent failure causes: (1) **length** — `verify_slice` strict-rejects any tag whose length ≠ `OutputSize` (32), and the client tag was 16 bytes, so verification failed before any byte comparison; (2) **algorithm** — even at matching length, XOR-fold bytes never equal HMAC-SHA256 bytes.
- The failure path was warn-only: `if mac.verify_slice(&sig).is_err() { warn!(...); }` with no kick, no drop — and the player's input was processed regardless (`last_input_seq`/`last_seen` updated, rate-limit deducted, snapshots continued).

W.0 disposition: **GO** — the documented defect shape held exactly; §1's locked decisions applied cleanly. (Recon also confirmed: zero existing tests on client/server behavior; the server already pulled `hmac`+`sha2`; the trio are full workspace members; baseline 53 proto tests green.)

### Doc-pointer note

The workflow prompt §4-W.4 named `docs/architecture/net.md` as the trace to update. The trio's signature defect is actually documented in **`docs/architecture/net_ecs.md`** (which traces the standalone trio + the dormant ECS-plugin layer); `net.md` traces the unrelated snapshot-based `astraweave-net`. `net_ecs.md` was updated instead — same intent (update the trace documenting this defect), corrected target. Recorded here rather than treated as a §7 STOP.

---

## 2. The remediation (commit ledger)

| Commit | Phase | What landed |
|---|---|---|
| `561b20957` | W.1 | Canonical HMAC-SHA256 surface in `aw-net-proto` (`SigningKey`, `sign`, `verify` constant-time, `hmac_sha256`, `input_frame_sig_payload`, `SIG_LEN=32`). `sign16` + `SessionKey` + `session_key_hint` **deleted**. `InputFrame.sig` widened `[u8;16]`→`[u8;32]`. RFC 4231 KAT + tamper/wrong-key/redaction tests. |
| `79424389e` | W.2.a | Client signs via canonical surface; `AW_SHARED_KEY` env config (fail-fast on malformed, dev-default + warn when unset); key material never logged. |
| `066cd6cfd` | W.2.b | Server verifies via canonical surface (both TLS + plain handlers, verify-FIRST before any state mutation); `SignatureFailurePolicy { Kick (default), Warn }`; Kick routes a Close(1008) through the existing disconnect/cleanup path; lib+bin split (`ServerConfig`, `spawn_server`); CLI/config plumbing. Inline `HmacSha256` + `hmac`/`sha2` direct deps removed. |
| `9a3fc94e3` | W.2.b-fix1 | Two server defects routed from Family-1 recon: both `.expect("room exists")` sites (hostile `JoinRoom` id / FindOrCreate re-lock race) → `ProtocolError` + clean end; snapshot-arm `?` early-returns → `warn!`+`break` so cleanup always runs. Mutation-verified regression tests. |
| `7029d7d7f` | W.3.1 | Family 1 (authenticated round-trip) + shared test harness (`tests/common/mod.rs`). |
| `a2b494942` | W.3.2 | Family 2 (tampered/malformed packets — reject-and-survive). |
| `0e702738e` | W.3.3 | Family 3 (wrong-key + policy behavior; default-is-Kick). |
| `68a9a1936` | W.3.4 | Family 4 (disconnect paths). |
| `420a6f61b` | W.5.1 | §5 fixes: `SigningKey` field made private (audited `from_bytes`/`as_bytes`); `from_hex` timing-boundary doc note; RFC 4231 KAT cases 3/4/6/7 added. |
| `2955cd14c` | W.5.2 | §5 fix: Family 5 (TLS signature-path coverage) — closes the CRITICAL gap that the default-TLS verify/kick path had zero tests. |
| `eb9977b88` | W.5.3 | §5 disposition: client design note documenting server→client asymmetric trust. |

**Final test tally** (baseline was 53 proto, 0 client, 0 server):

| Crate | Tests | Breakdown |
|---|---|---|
| `aw-net-proto` | 59 | 1 integration file (was 53; −12 deleted sign16/SessionKey, +14 HMAC/tamper/redaction, +4 RFC 4231 cases) |
| `aw-net-server` | 41 | family1=5, family2=12, family3=9, family4=9, family5=4, w2b_fix1=2 |
| `aw-net-client` | 4 | family1_client_binary=2, family3_client_binary_wrong_key=2 |
| **Total** | **104** | all green |

---

## 3. §5 adversarial refute pass

Two refute teams (crypto + test-baseline), 10 diverse lenses + a completeness critic, each finding independently re-verified by an agent holding the opposite prior. **22 raw findings → 5 confirmed in-scope, 7 confirmed fenced/known-limitations, 10 rejected.** Iterated to **CONVERGENCE** (the post-fix fix-verifier confirmed all 5 closed, no new in-scope defects, gates green).

### 3.1 Confirmed in-scope findings and dispositions

| # | Finding | Sev | Disposition |
|---|---------|-----|-------------|
| 2 | `SigningKey(pub [u8;32])` — the public field let any code bypass the redacted `Debug` via `.0` (the test/bench did exactly `hex::encode(key.0)`). | HIGH | **FIXED** (W.5.1). Field private; raw bytes reachable only via the named, greppable `as_bytes()`; `from_bytes()` is the explicit constructor; `Debug` stays redacted. Verified: no `Display`/`Deref`/`Serialize`/`AsRef` leak surface added. |
| 3+4 | The TLS verify path (`on_client_msg_tls`) — the **default** production mode (`tls_enabled` defaults true) — had **zero** test coverage; the "both handlers semantically identical" security claim was unverified. | CRIT/HIGH | **FIXED** (W.5.2). Family 5 drives the real `accept_loop_tls`→`handle_socket_tls`→`on_client_msg_tls` path over `wss://`: signed round-trip under Kick, wrong-key kick (explicit Close 1008 + reason), and Warn-stays-open. Non-vacuity mutation-verified. |
| 1 | `from_hex` uses non-constant-time `hex::decode`. | LOW | **DOCUMENTED** (W.5.1 doc note). Key material is operator-supplied out-of-band config parsed **once at startup** (`AW_SHARED_KEY` / `--shared-key-hex`), never attacker-reachable input — there is no chosen-input timing oracle. Hand-rolled constant-time hex would be fragile crypto-adjacent code for negligible benefit. `hex::decode` retained; rationale recorded in the `from_hex` doc-comment. |
| 5 | Client does not verify server→client packets (asymmetric: server verifies all client `InputFrame`s, client trusts all server messages). | MED | **DOCUMENTED** (W.5.3 client design note). This is an authoritative-server trust model: the server defines truth and the client has no independent ground truth. Critically, a *shared symmetric* key gives **no coherent** S2C authentication anyway — every client in a room holds the same key, so a symmetric MAC cannot prove "from the server" vs "from a peer." Meaningful S2C auth needs asymmetric server keys or per-session key exchange — both **fenced** (§1.3 handshake/key-exchange out of scope). |

Both `documented` dispositions were the resolution explicitly sanctioned by the finding's own independent verifier; neither is a silent baseline relaxation (§2) — each carries recorded rationale in code and here.

### 3.2 Confirmed fenced / known-limitations (routed here, not fixed)

1. **Replay protection / nonces / sequence-number authentication — NOT implemented.** A validly-signed `InputFrame` captured off the wire can be replayed; the HMAC proves authenticity, not freshness. Explicitly out of scope (§2). Future session-security work.
2. **Server→client authentication — not provided** (see §3.1 #5). Needs asymmetric/per-link keys = fenced handshake work.
3. **`from_hex` non-constant-time** (see §3.1 #1) — accepted boundary for operator-supplied startup keys.
4. **Shipped client binary over TLS — untested.** The shipped `aw-net-client` defaults to `wss://` but uses `native-tls`, which rejects the self-signed dev cert; a cert-validation/handshake concern (fenced). Note: the **server** TLS verify/kick/Warn path IS now covered by Family 5 (via a test-only rustls client) — only the shipped binary's TLS leg is uncovered.
5. **Test-assertion timing comparisons** — proto tests use `assert_eq!`/`assert_ne!` on 32-byte tags/keys. Test infrastructure, not the production crypto path (which is constant-time). Fenced as test-infra best-practice.
6. **TLS-path RateLimited coverage / fixed 50 ms test poll interval / non-deterministic select-arm in family4 test 2** — accepted test-design properties; assert observable outcomes, not internal timing.

### 3.3 Rejected findings (10)

The refute surfaced and the verification rejected, among others: a false "test code is a timing-attack vector" (tests don't process untrusted input); "RFC 4231 incomplete" (impl correct; cases nonetheless added in W.5.1 as strengthening); several "TLS untested" duplicates already covered by Family 5; "benchmarks access `.0`" (moot — field now private); and three flakiness claims (raw-sleep soak, tick-monotonicity, `Instant` deadline) all correctly rejected on monotonic-clock / large-safety-margin / output-based-assertion grounds. Full text in the workflow result transcript.

---

## 4. Closure proofs (§6)

| # | Proof | Status |
|---|---|---|
| 1 | `cargo test` green across all three trio crates; `cargo check --workspace` green | PASS (104 trio tests; workspace clean — only pre-existing unrelated warnings) |
| 2 | Valid-key round-trip: zero verification failures (Family 1) | PASS |
| 3 | Tampered/malformed inputs rejected without server panic (Family 2) | PASS |
| 4 | Wrong-key under default kicks via real disconnect; under Warn logs-and-continues (Family 3) | PASS |
| 5 | Disconnect-path family green (Family 4) | PASS |
| 6 | No XOR-stub/vestigial signing primitive in the trio (grep-clean) | PASS (only residue was stale `net/README.md` prose, fixed in W.4; `xorshift32` in a test is an unrelated PRNG) |
| 7 | Constant-time tag verification at every verify site | PASS (single `aw_net_proto::verify` → `Mac::verify_slice`; both server handlers route through it; no `==`/truncated-verify anywhere) |
| 8 | Both refute teams converged, no outstanding in-scope findings | PASS |
| 9 | W.0 regression baseline holds | PASS (the 53 baseline proto tests' intent preserved; sign16/SessionKey tests superseded per this record) |
| 10 | Docs landed; commits pushed to origin/main; tree clean | (closed by this commit + push) |

---

## 5. Deferred-items log (observed, not fixed — single-concern discipline, §2)

Logged for future campaigns; none are the signature defect or load-bearing for it.

- **Rate-limit toothlessness**: server sends `ServerToClient::RateLimited` without disconnecting; the client logs and continues. Pre-existing, fenced.
- **`let _ = ws.send(Message::Pong(p))`** in both server connection loops (discarded `Result`).
- **Committed dev certs are placeholders**: `net/certs/dev/dev-cert.pem`/`dev-key.pem` are placeholder text, not real X.509/PKCS#8. A default (`tls_enabled: true`) server will not start until real certs are generated via `net/certs/dev/generate_dev_cert.{sh,ps1}`. Family 5 sidesteps this by generating ephemeral `rcgen` certs per-test. Intentional (cert material should not be committed); documented here so operators know to generate certs.
- **Declared-but-unused Cargo deps**: `aw-net-proto` (`time`), `aw-net-server` (`tungstenite` direct, `hyper`, `serde_json`, `time`, `thiserror`), `aw-net-client` (`tungstenite`, `url`).
- **Dead sled DB**: `AppState.db` is `#[allow(dead_code)]` — opened, never read/written.
- **Per-connection `build_snapshot`** increments `room.tick`/`snap_id`, so snapshot tick rate scales with player count.
- **`astraweave-net-ecs/src/lib_temp.rs`** near-duplicate of `lib.rs` (read-only this workflow; Dormant Surface Disposition's to resolve).
- **Unrelated workspace warnings** surfaced incidentally during the campaign (editor `gizmo/mod.rs` unused `TranslateGizmo` import; `astraweave-render` `bloom` `cfg` feature; assorted unused test imports) — out of scope, untouched.

---

## 6. Forward chain

Per the workflow's §8: **HARD STOP at closure.** No successor sub-phase. The known-limitations (§3.2 — replay protection, server→client authentication, shipped-client-over-TLS, real cert distribution) feed any future **session-security** work, and the dormant-surface notes (two coexisting networking subsystems; `lib_temp.rs`) feed the future **Dormant Surface Disposition** campaign. Both are Andrew-routed; neither is started here.
