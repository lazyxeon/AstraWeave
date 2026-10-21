# Multi-Tool 5.C Closeout — 2026-07-22

## Outcome

Multi-Tool Sub-phase 5 is **COMPLETE**. SP5.A `2f6f853a5` and SP5.B `9bae816e3` landed 2026-06-06; the required §7.4 Andrew-gate PASS was declared 2026-07-20 and is recorded in `T_SERIES_RATIFICATION_2026-07-20.md` §1. G-pointer-events-fix is **SUBSUMED**, not partial: its post-pause requirements were the real `RegionalArchetypePanel` `ActiveTool` implementation plus dispatcher registration, both shipped in SP5.B. RAV remains paused through Multi-Tool Mediator Removal + SP6, then resumes at H-saveload-diagnostic.

## Evidence

1. **Permanent real-panel guard — `2e92b8a3b`.** `real_panel_paint_click_drag_consumes_pointer_events` constructs `RegionalArchetypePanel::new_dispatch_tool`, asserts press + drag + release each return `EventDisposition::Consumed`, and checks the emitted stroke sequence. Focused: **1 passed; 4023 filtered out**. Panel module: **31 passed; 3993 filtered out**. Dispatcher module: **21 passed; 4003 filtered out**. Full `aw_editor` library: **4019 passed; 0 failed; 5 ignored; 0 filtered out**. `cargo check -p aw_editor` built successfully (one pre-existing unused-import warning).
2. **Doc-lint allowlist — `6dad06ccb`.** Seven director-approved rows cover seven citation/quote findings (the duplicated CI-workshop alias occurrence shares one parser key); source documents were not reworded. `aw_doc_lint --mode warn`: **94 literals; 581 files scanned; 633 matches; 0 un-allowlisted; K=0 clean**.
3. **Attribution merge fix — `5778fd17b`.** `generate_attribution_file()` now reads the existing provider file, unions entries by handle, replaces same-identity entries with current metadata, emits deterministic handle/license ordering, and preserves the existing footer/tail note. Malformed existing files fail closed instead of being overwritten. The committed surface was confirmed as `assets/_downloaded/polyhaven/ATTRIBUTION.txt`; it was inspected but not mutated. Focused union + twice-run idempotence fixture: **1 passed; 124 filtered out**. Full offline crate suite: library **125 passed**; integration **9 passed**; API **39 passed**; download integration **8 passed**; mutation-resistant **46 passed**; PolyHaven API fixtures **44 passed**; all zero failures. `cargo check -p astraweave-assets` built successfully.

## Commits and scope

- `2e92b8a3b` — real-panel pointer-consumption regression guard.
- `5778fd17b` — attribution union/idempotence implementation and fixture.
- `6dad06ccb` — seven allowlist rows.
- This commit — campaign §11/§12, RAV pointer, and this outcome note.

No mediator removal, SP6 implementation, RAV feature work, editor fine-tuning, provider-flow refactor, manifest/fetch work, asset/release mutation, terrain change, or water change was performed.
