# Master Attribution & Licenses

This document centralizes attribution and licensing for third‑party code, tools, and content used by AstraWeave. It is kept concise and actionable; for full dependency inventories, see the automation notes below.

Last updated: 2025‑11‑06

---

## How to comply (TL;DR)

- Keep this file up to date when adding any third‑party code, model, texture, audio, or tool.
- Prefer sources with permissive or CC0 licenses for bundled assets.
- If the license requires attribution, add an entry in the relevant section with author, license, and source URL.
- If an asset’s license is unknown, do not ship it. Add it to the “Unverified/Action required” section and replace it.

Automation (recommended): generate a full license list of crates before releases and archive it under `docs/current/compliance/`.

```bash
# Optional: generate third‑party crate licenses (example tooling)
cargo install cargo-license
cargo license --json > docs/current/compliance/cargo-licenses.json
```

---

## Code dependencies (major)

Most Rust crates in this repository are dual‑licensed MIT/Apache‑2.0. Always consult the crate’s README for the authoritative license.

- wgpu — MIT/Apache‑2.0 — https://github.com/gfx-rs/wgpu
- Bevy — MIT/Apache‑2.0 — https://github.com/bevyengine/bevy
  - Note: Renderer foundation used via our `astraweave-render-bevy` integration.
- glam — MIT/Apache‑2.0 — https://github.com/bitshifter/glam-rs
- rapier (Rapier3D) — MIT/Apache‑2.0 — https://github.com/dimforge/rapier
- egui — MIT/Apache‑2.0 — https://github.com/emilk/egui
- rodio — MIT/Apache‑2.0 — https://github.com/RustAudio/rodio
- serde — MIT/Apache‑2.0 — https://github.com/serde-rs/serde
- tokio — MIT/Apache‑2.0 — https://github.com/tokio-rs/tokio
- rayon — MIT/Apache‑2.0 — https://github.com/rayon-rs/rayon
- anyhow — MIT/Apache‑2.0 — https://github.com/dtolnay/anyhow
- thiserror — MIT/Apache‑2.0 — https://github.com/dtolnay/thiserror
- gltf (Rust loader) — MIT/Apache‑2.0 — https://github.com/gltf-rs/gltf
- tracy (client/integration) — BSD‑3‑Clause (upstream) / MIT/Apache‑2.0 (Rust bindings) — https://github.com/wolfpld/tracy

If additional major frameworks are introduced (e.g., Bevy), add them here with links and licenses.

---

## Media: textures, HDRIs, materials

- Poly Haven — CC0 1.0 Universal — https://polyhaven.com/
  - Status: Used for HDRIs/materials via manifests and manual workflow.
  - Attribution: Not required (CC0), appreciated; include source link when practical.

If you add non‑CC0 textures or HDRIs, include author, license, and URL here and in your asset README.

---

## 3D models and sample content

- Kenney.nl — CC0 1.0 Universal — https://kenney.nl/
  - Status: Provider manifests exist; direct downloads may be manual. Credit: “Kenney.nl (CC0)” with link.

- Quaternius — CC0 1.0 Universal — https://quaternius.com/
  - Status: Manifests/workflow supported; credit: “Quaternius (CC0)” with link.

- Poly Pizza — Aggregator; license varies per asset — https://poly.pizza
  - Status: Use asset‑specific licenses; always link individual asset pages and authors.
  - Template entry (add per asset used):
    - Asset: <Name>
    - Author: <Creator>
    - License: <License>
    - Source: <URL>

> IMPORTANT: Do not ship third‑party models with unclear or non‑permissive licenses.

---

## Audio assets

- If you add SFX/music, list each pack/track with author, license, and URL here.
- Prefer CC0 or permissive licenses for bundled audio.

Template:
- Track/Pack: <Name> — Author: <Author> — License: <License> — Source: <URL>

---

## Tools, services, and models

- Ollama / Hermes 2 Pro (Nous Research) — External model; consult model card/license
  - Status: Integrated for local inference in examples; weights not distributed by this repo.
  - Link: https://ollama.ai/ and https://huggingface.co/NousResearch/hermes-2-pro

- Tracy Profiler — BSD‑3‑Clause — https://github.com/wolfpld/tracy

If you integrate cloud services or additional LLMs, add usage notes and their licenses here.

---

## Unverified/Action required

- `assets/demo_plane.gltf` — Origin/license unverified.
  - Action: Replace with a CC0 example model (e.g., Kenney/Quaternius) or document source/license here.

If any other bundled asset lacks a clear license, remove it or document it here until resolved.

---

## Per‑asset credit template (copy/paste)

```md
- Asset: <Name>
  - Author: <Creator>
  - License: <License name + short URL>
  - Source: <https://...>
  - Notes: <modifications (if any)>
```

---

## Update workflow

1. For new crates: verify license in the crate README; if it’s non‑standard, add it to the list above.
2. For new assets: record author, license, and URL when you download/import. Prefer CC0.
3. For releases: generate a current crate license inventory and archive it under `docs/current/compliance/`.
4. Remove or replace any unverified assets prior to tagging a release.

— End of file —
