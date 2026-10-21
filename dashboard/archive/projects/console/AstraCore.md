
# AstraCore — Companion‑First AI Console

**AstraCore** is a console platform designed for **AI‑native gameplay**. It hosts the player’s companion AI **as a resident service**, accelerates on‑device inference, and ships a low‑latency OS + SDK so games built on AstraWeave feel instantaneous and fair.

---

## 1) Hardware Targets (reference design)

- **CPU**: 8–12 cores (ARM64 or x86), strong single‑thread perf for sim.  
- **GPU**: 10–20 TFLOPS FP32, Vulkan‑first, modern upscaler support.  
- **NPU**: 100–200 TOPS INT8 (FP16/bfloat16 paths) for on‑device LLM + policy nets.  
- **Memory**: 32–64 GB unified; **Storage**: 1–2 TB NVMe Gen4.  
- **I/O**: Wi‑Fi 7, BT 5.4, 2×USB‑C, HDMI 2.1.  
- **Latency**: ≤150 ms for companion intent; ≤80 ms input→action in combat.

---

## 2) OS & Services

- **OS**: Linux LTS, low‑latency kernel; immutable base; OTA A/B updates.  
- **Graphics/Audio**: Vulkan, PipeWire; **Net**: QUIC‑first; rollback‑safe sessions.  
- **Secure boot + TEE**: protects model slots and companion profiles.  
- **Companion Daemon**: always‑on process hosting the user’s AI (IPC via gRPC/protobuf).  
- **AI Runtime**: local inference first; cloud connector (opt‑in) for deep planning.  
- **Policy Engine**: system‑wide filters, parental controls, profanity/harassment safeguards.

---

## 3) Platform Contracts

- **Companion Portability**: one `.cprof` per user; encrypted, signed, portable across AstraWeave games.  
- **Tool Contract Registry**: per‑title verb sets validated by the OS; prevents exploits.  
- **Director Budgeting**: per‑title caps for boss terrain edits/spawns enforced by platform.

---

## 4) Creator & Store

- **Marketplace**: personas, voice packs, boss modules, maps (strict curation; no P2W).  
- **Rev share**: 90/10 in favor of creators.  
- **Moderation**: automated + human review; zero tolerance for abusive content.

---

## 5) Dev Kit

- **SDK**: schemas (snapshots, intents, profiles), C/C++/Rust bindings, Godot adapter.  
- **Testing**: headless fuzzer, replay harness, perf dashboards.  
- **Certification**: determinism tests, sandbox integrity, profanity & safety checks.

---

## 6) Privacy & Data

- **Default**: on‑device only; cloud disabled.  
- **Explicit Opt‑In**: users can enable cloud deep‑think per title; clear data retention UI.  
- **Telemetries**: anonymized, aggregate; never include raw player voice.

---

## 7) Launch Plan (indicative)

- **Phase 1**: PC dev kit + Veilweaver vertical slice (one biome, one adaptive boss).  
- **Phase 2**: Alpha console board; hit local inference latency targets.  
- **Phase 3**: Partner pilots (3–5 teams), Creator marketplace soft‑open.  
- **Phase 4**: Public reveal and preorders with Veilweaver showcase.

---

## 8) Philosophy

- We sell **games**, not gambling mechanics.  
- We ship **AI that obeys the same rules** as players — no cheating.  
- We build **companionship** as a platform feature, not a gimmick.
