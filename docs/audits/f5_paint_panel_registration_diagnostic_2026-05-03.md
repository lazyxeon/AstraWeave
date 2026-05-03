# F.5-paint Panel Registration Diagnostic

**Date**: 2026-05-03
**Trigger**: F.5-paint Andrew-gate REGRESS verdict (no "Regional Archetype" panel visible in editor's View/Window menu).
**Predecessor work**: F.5-paint commits `26a3864b8` (paint.A scaffold) → `226572bae` (paint.B brush) → `2b230d94e` (paint.C save/load) → `e9d2a7922` (paint.D closeout) → `b6dd9de58` (hash-fixup).
**Scope**: diagnostic only — root cause identification + recommended remediation approach. **No production code changes**. The remediation lands in F.5-paint.F-fix as a separate session per Andrew's Q1 resolution.

---

## §1 — Andrew-gate REGRESS observations

Andrew opened the editor at the F.5-paint commit chain end (hash-fixup `b6dd9de58`). Reported observations:

- **No "Regional Archetype" panel option** in the editor's View → "Panels" submenu (where the canonical "Terrain", "Hierarchy", "Inspector", etc., entries live).
- Click+drag pointer events on the viewport registered as undo entries on the **existing Terrain panel's** sculpt/paint brush (status bar showed "Terrain Paint" / "Terrain Sculpt" entries — not anything related to RegionalArchetype).
- The "Save Mask" / "Load Mask" buttons that Andrew clicked were on the **existing Terrain panel** (not the F.5-paint panel) — those buttons appeared to have no visible effect, but that observation is misdirected: the F.5-paint panel was never reachable, so its Save/Load wiring was not the buttons being exercised.

The observations are consistent with the panel struct existing but never being instantiated by the editor's UI construction code.

---

## §2 — Investigation methodology

The F.5-paint.E-diagnostic prompt §2.1 enumerated five hypotheses (A-E). Hypothesis A (lowest-hanging fruit; "Module declared but registration call never made") was investigated first. The investigation procedure was:

1. **Inspect F.5-paint.A's actual diff** to determine which files it modified beyond the new panel file itself.
2. **Search the entire `tools/aw_editor/src/` directory tree** for any reference to `RegionalArchetypePanel`, `regional_archetype_panel`, or `RegionalArchetype` outside the panel's own file.
3. **Map the canonical "register a new panel" pattern** by tracing how the existing `TerrainPanel` (visible in Andrew's screenshots) is wired into the editor's panel system. Identify every distinct edit-site a new panel must touch.
4. **Compare F.5-paint.A's footprint against the canonical pattern** to determine which surfaces were missed.
5. If Hypothesis A is confirmed, B-E need only short rule-out paragraphs (since A is sufficient root cause); the diagnostic does not chase deeper.

Hypothesis A was confirmed by step 2 (search returned zero hits) and step 4 (F.5-paint.A modified only `panels/mod.rs:54` plus the new panel file). The investigation proceeded to map the canonical pattern in full detail (since F.5-paint.F-fix's prompt-drafting needs that map).

---

## §3 — Findings

**Root cause**: **Hypothesis A — Module declared but registration call never made.**

F.5-paint.A added `pub mod regional_archetype_panel;` at [tools/aw_editor/src/panels/mod.rs:54](../../tools/aw_editor/src/panels/mod.rs#L54), making the new panel module reachable from Rust's compilation graph. But it added zero registration calls. The panel struct exists, compiles, has 30 passing unit tests — and is never instantiated by the editor's UI construction code, so it cannot appear in any user-facing panel registry, menu, or tab.

**Evidence (specific code references)**:

1. **F.5-paint.A's actual diff** (`git show --stat 26a3864b8`):
   - `tools/aw_editor/src/panels/mod.rs` (+1 line — the `pub mod regional_archetype_panel;` declaration).
   - `tools/aw_editor/src/panels/regional_archetype_panel.rs` (+333 lines, new file).
   - **Nothing else.** No edits to `panel_type.rs`, `tab_viewer/mod.rs`, `ui/menu_bar.rs`, or `main.rs`.

2. **F.5-paint.B/C diffs** (`git show --stat 226572bae 2b230d94e`): both touched only `regional_archetype_panel.rs` (extending the file with brush logic and save/load API). Neither added any registration call.

3. **Workspace-wide grep for `RegionalArchetype`** in `tools/aw_editor`:
   - One file matches: `tools/aw_editor/src/panels/regional_archetype_panel.rs` (the panel's own file).
   - Plus the single `pub mod regional_archetype_panel;` line in [tools/aw_editor/src/panels/mod.rs:54](../../tools/aw_editor/src/panels/mod.rs#L54).
   - **Zero other matches anywhere in the workspace.** No `PanelType::RegionalArchetype` variant exists; no `regional_archetype_panel: RegionalArchetypePanel` field exists; no `Self::regional_archetype_panel.show(ui)` dispatch arm exists.

**Hypotheses B, C, D, E rule-out** (brief, since Hypothesis A is sufficient root cause):

- **Hypothesis B** (registration call made but at wrong location): ruled out — no registration call exists at all (per evidence 3 above), so it cannot be at a wrong location.
- **Hypothesis C** (registration via separate mechanism F.5-paint.A bypassed): ruled out — the editor uses straightforward struct-field + match-arm registration (no macro / no config-file discovery / no plugin system). Existing panels follow the same pattern; F.5-paint.A just didn't follow it.
- **Hypothesis D** (`Panel` trait implementation incomplete): ruled out — the `Panel` trait is minimal (`name() -> &str` + `show(&mut Ui)` + optional `update()`); F.5-paint.A's impl at [tools/aw_editor/src/panels/regional_archetype_panel.rs:459](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L459) is complete. The trait itself is irrelevant to menu population — the menu reads `PanelType::all()`, not the panel struct.
- **Hypothesis E** (multiple registration paths, F.5-paint.A used one but not all): partially applicable — the canonical pattern *does* have multiple registration surfaces (10 of them; see §4 below). F.5-paint.A engaged exactly one of them (the `pub mod` declaration). But characterizing this as "used one but not all" understates the gap: 9 of 10 surfaces were missed entirely, including the load-bearing one (the `PanelType` enum variant). Hypothesis A's framing is more accurate: registration was effectively never attempted.

---

## §4 — Existing panel registration pattern (reference)

Adding a new editor panel requires touching **10 distinct code surfaces** across **2 files** (with one additional re-export in `panels/mod.rs`). The TerrainPanel and the more recent BlueprintPanel both follow this pattern; the BlueprintPanel is the closest reference example since it was added under an analog "new panel for a new feature" workflow.

### §4.1 — `tools/aw_editor/src/panel_type.rs` (7 surfaces)

The `PanelType` enum is the source-of-truth for the editor's panel registry. Three user-facing surfaces (View/Window menu, "Add Panel" popup, console panel-list command) iterate `PanelType::all()` to build their UI; if a panel struct is not represented in this enum, it is structurally unreachable from any user-facing surface. The 7 surfaces inside `panel_type.rs` are:

1. **Enum variant** at [panel_type.rs:107-228](../../tools/aw_editor/src/panel_type.rs#L107). Add a variant: `RegionalArchetype,` (new doc comment per existing convention).
   - Reference: [`Blueprint,` at line 227](../../tools/aw_editor/src/panel_type.rs#L227).
   - Reference: [`Terrain,` at line 166](../../tools/aw_editor/src/panel_type.rs#L166).

2. **`title()` match arm** at [panel_type.rs:234-278](../../tools/aw_editor/src/panel_type.rs#L234). Add: `Self::RegionalArchetype => "Regional Archetypes"`.
   - Note: `RegionalArchetypePanel::name()` already returns `"Regional Archetypes"`; keep these consistent.
   - Reference: [`Self::Blueprint => "Blueprint"` at line 276](../../tools/aw_editor/src/panel_type.rs#L276).
   - Reference: [`Self::Terrain => "Terrain"` at line 255](../../tools/aw_editor/src/panel_type.rs#L255).

3. **`icon()` match arm** at [panel_type.rs:283-327](../../tools/aw_editor/src/panel_type.rs#L283). Add: `Self::RegionalArchetype => "[RA]"` (or another distinct 2-char code; existing icons are 1-3 chars in brackets).
   - Reference: [`Self::Blueprint => "[BP]"` at line 325](../../tools/aw_editor/src/panel_type.rs#L325).
   - Reference: [`Self::Terrain => "[Tr]"` at line 304](../../tools/aw_editor/src/panel_type.rs#L304).

4. **`category()` match arm** at [panel_type.rs:354-409](../../tools/aw_editor/src/panel_type.rs#L354). Add `Self::RegionalArchetype` to the `PanelCategory::Content` arm (the panel is content-creation: paintable archetype mask authoring).
   - Reference: [`| Self::Terrain` in the Content arm at line 395](../../tools/aw_editor/src/panel_type.rs#L395) — Terrain is precedent for treating terrain-authoring as Content.
   - Alternative: `PanelCategory::Tools` (where `Blueprint` lives) — Tools and Content are both reasonable; pick whichever the F.5-paint.F-fix prompt prefers. Recommendation: Content (matches Terrain precedent for terrain-related authoring).

5. **`description()` match arm** at [panel_type.rs:411-457](../../tools/aw_editor/src/panel_type.rs#L411). Add: `Self::RegionalArchetype => "Paint regional archetype mask for per-region terrain shape variation"` (or similar).
   - Reference: [`Self::Blueprint => "2D top-down zone editor for defining terrain generation zones"` at line 455](../../tools/aw_editor/src/panel_type.rs#L455).

6. **`all()` slice** at [panel_type.rs:494-538](../../tools/aw_editor/src/panel_type.rs#L494). Add `Self::RegionalArchetype,` to the slice.
   - Reference: [`Self::Blueprint,` at line 536](../../tools/aw_editor/src/panel_type.rs#L536).
   - **Critical surface**: `PanelType::all()` is consumed by all three user-facing panel-discovery sites (§4.4 below). Missing this entry is the single most load-bearing failure mode.

7. **(Optional) `has_scroll()` match arm** at [panel_type.rs:343-352](../../tools/aw_editor/src/panel_type.rs#L343). Default is `true` (panels scroll); add `Self::RegionalArchetype` to the false-arm only if the panel needs custom pan/zoom (it does not for F.5-paint scope; the panel uses `egui::ScrollArea::vertical` per [regional_archetype_panel.rs:467](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L467) and is consistent with the default scrolling behavior). **Skip this surface for F.5-paint.F-fix.**

### §4.2 — `tools/aw_editor/src/tab_viewer/mod.rs` (3 surfaces)

The `EditorTabViewer` struct holds the actual panel instances. To get the `RegionalArchetypePanel` instance into the runtime tab dispatch, three changes are needed:

1. **Field on the `EditorTabViewer` struct** at [tab_viewer/mod.rs:665](../../tools/aw_editor/src/tab_viewer/mod.rs#L665) (or near the other panel fields; keep consistent with the existing alphabetical-ish grouping under the `// === New Phase 8 SOTA Panels ===` comment block at line 666). Add: `regional_archetype_panel: RegionalArchetypePanel,`.
   - Reference: [`terrain_panel: TerrainPanel,` at line 665](../../tools/aw_editor/src/tab_viewer/mod.rs#L665).
   - Reference: [`blueprint_panel: crate::panels::BlueprintPanel,` at line 706](../../tools/aw_editor/src/tab_viewer/mod.rs#L706).

2. **Initializer in `EditorTabViewer::new()`** at [tab_viewer/mod.rs:963](../../tools/aw_editor/src/tab_viewer/mod.rs#L963). Add: `regional_archetype_panel: RegionalArchetypePanel::new(),`.
   - Reference: [`terrain_panel: TerrainPanel::new(),` at line 963](../../tools/aw_editor/src/tab_viewer/mod.rs#L963).
   - Reference: [`blueprint_panel: crate::panels::BlueprintPanel::new(),` at line 984](../../tools/aw_editor/src/tab_viewer/mod.rs#L984).

3. **Match arm in the tab-render dispatch** at the `PanelType` match block starting around [tab_viewer/mod.rs:7620](../../tools/aw_editor/src/tab_viewer/mod.rs#L7620). Add:
   ```rust
   PanelType::RegionalArchetype => {
       use crate::panels::Panel;
       self.regional_archetype_panel.show(ui);
   }
   ```
   - Reference: [`PanelType::Terrain => { ... self.terrain_panel.show(ui); ... }` at lines 7620-7639](../../tools/aw_editor/src/tab_viewer/mod.rs#L7620). Note Terrain has additional action-pump logic for `take_actions()`; F.5-paint's panel doesn't expose actions yet (no `pending_paint_ops` action enum), so the simpler arm shape from `PanelType::UiEditor` at [line 7641](../../tools/aw_editor/src/tab_viewer/mod.rs#L7641) is the correct template.

### §4.3 — `tools/aw_editor/src/tab_viewer/mod.rs` use statement

The `use crate::panels::{...}` statement at [tab_viewer/mod.rs:45](../../tools/aw_editor/src/tab_viewer/mod.rs#L45) imports the panel types. Add `RegionalArchetypePanel` to that list (or another nearby `use crate::panels::*` if one exists). This is required for the field type and `::new()` call in §4.2 to resolve.

### §4.4 — Confirmation: `PanelType::all()` is the menu-population source of truth

Three surfaces consume `PanelType::all()` to populate user-facing panel-discovery UI:

1. [`tools/aw_editor/src/ui/menu_bar.rs:368`](../../tools/aw_editor/src/ui/menu_bar.rs#L368) — populates the View → "Panels" submenu. **This is the surface Andrew's REGRESS observed**.
2. [`tools/aw_editor/src/tab_viewer/mod.rs:475`](../../tools/aw_editor/src/tab_viewer/mod.rs#L475) — populates the "Add Panel" popup menu (alternative panel-discovery surface).
3. [`tools/aw_editor/src/main.rs:5941`](../../tools/aw_editor/src/main.rs#L5941) — populates the console's panel-list command (debug surface).

All three iterate `PanelType::all()`. None reflect on the panel structs in `tools/aw_editor/src/panels/`. So the `PanelType` enum is the load-bearing source of truth; without a variant there, the panel cannot surface anywhere user-facing.

### §4.5 — Existing canonical reference: `BlueprintPanel`

`BlueprintPanel` is the most recent panel addition that followed the full pattern correctly. It demonstrates a "new panel for a new feature" workflow analogous to F.5-paint's intended scope. Its registration touchpoints (commit history not investigated, but current state):

- Enum variant: [`Blueprint,` at panel_type.rs:227](../../tools/aw_editor/src/panel_type.rs#L227).
- Title: [`Self::Blueprint => "Blueprint"` at panel_type.rs:276](../../tools/aw_editor/src/panel_type.rs#L276).
- Icon: [`Self::Blueprint => "[BP]"` at panel_type.rs:325](../../tools/aw_editor/src/panel_type.rs#L325).
- Category: in `Tools` arm at [panel_type.rs:383](../../tools/aw_editor/src/panel_type.rs#L383).
- Description: at [panel_type.rs:455](../../tools/aw_editor/src/panel_type.rs#L455).
- `all()` slice: at [panel_type.rs:536](../../tools/aw_editor/src/panel_type.rs#L536).
- `has_scroll()` false-arm: at [panel_type.rs:348](../../tools/aw_editor/src/panel_type.rs#L348) (Blueprint uses canvas pan/zoom; F.5-paint does NOT need this).
- Field: `blueprint_panel: crate::panels::BlueprintPanel,` at [tab_viewer/mod.rs:706](../../tools/aw_editor/src/tab_viewer/mod.rs#L706).
- Initializer: `blueprint_panel: crate::panels::BlueprintPanel::new(),` at [tab_viewer/mod.rs:984](../../tools/aw_editor/src/tab_viewer/mod.rs#L984).
- Tab dispatch: not searched but presumed to exist near the `PanelType::Terrain` arm.

F.5-paint.F-fix should mirror this pattern exactly, substituting `RegionalArchetype` / `RegionalArchetypePanel` / `regional_archetype_panel` for the `Blueprint` analog.

---

## §5 — Recommended remediation approach

F.5-paint.F-fix should land **a single commit** (or two if the test pattern is non-trivial) with the following changes:

### §5.1 — Code changes

**File 1: `tools/aw_editor/src/panel_type.rs`** — 6 edits:

1. Add `RegionalArchetype` enum variant (with doc comment) at the bottom of the `PanelType` enum, near `Blueprint`. Group it with the other content-creation panels (after `BlendImport,` and before `Blueprint,` matches the existing groupings; adjacent placement is cosmetic but improves readability).
2. Add `Self::RegionalArchetype => "Regional Archetypes"` to the `title()` match arm.
3. Add `Self::RegionalArchetype => "[RA]"` to the `icon()` match arm. Verify `[RA]` doesn't collide with any existing icon (none currently use the `RA` digraph; safe).
4. Add `Self::RegionalArchetype` to the `PanelCategory::Content` arm in `category()` (matching `Terrain` precedent).
5. Add `Self::RegionalArchetype => "Paint regional archetype mask to define per-region terrain shape variation"` (or similar; one-line) to the `description()` match arm.
6. Add `Self::RegionalArchetype,` to the `all()` slice at the bottom (after `Blueprint,`).

**File 2: `tools/aw_editor/src/tab_viewer/mod.rs`** — 4 edits:

1. Add `RegionalArchetypePanel` to the `use crate::panels::{...}` statement at [line 45](../../tools/aw_editor/src/tab_viewer/mod.rs#L45) (or keep with the other `crate::panels::Foo` qualified usages if simpler).
2. Add `regional_archetype_panel: RegionalArchetypePanel,` field on the `EditorTabViewer` struct (around [line 705](../../tools/aw_editor/src/tab_viewer/mod.rs#L705); near `blueprint_panel`).
3. Add `regional_archetype_panel: RegionalArchetypePanel::new(),` initializer in `EditorTabViewer::new()` (around [line 984](../../tools/aw_editor/src/tab_viewer/mod.rs#L984); near `blueprint_panel:`).
4. Add the `PanelType::RegionalArchetype => { use crate::panels::Panel; self.regional_archetype_panel.show(ui); }` match arm in the render dispatch (around [line 7641](../../tools/aw_editor/src/tab_viewer/mod.rs#L7641)+, near `PanelType::UiEditor`'s simpler-shaped arm).

### §5.2 — Regression test (Pattern A recommended; Pattern B deferred)

Per Andrew's Q2 resolution: prefer Pattern B (full editor instantiation test) where achievable; fall back to Pattern A (panel registry membership test). For F.5-paint.F-fix:

**Pattern B feasibility analysis**:

- The editor's `EditorTabViewer::new()` is the panel-instantiation site. It does not require a live `egui::Context` or wgpu surface; it constructs panel structs directly. A unit test could call `EditorTabViewer::new()` and assert `tab_viewer.regional_archetype_panel.name() == "Regional Archetypes"`.
- However, this test verifies struct-instantiation only — not menu-population. Andrew's REGRESS was at the menu-population layer. To verify menu population, the test must call `PanelType::all()` and assert the new variant is present.
- **A combined Pattern A + Pattern B test** captures both layers: instantiate `EditorTabViewer`, then assert (a) `PanelType::all().contains(&PanelType::RegionalArchetype)`, and (b) `EditorTabViewer::new()` produces a struct whose `regional_archetype_panel` field holds a panel whose `name()` returns `"Regional Archetypes"`.

**Recommended test shape** (Pattern A + light-Pattern-B; lands in `tools/aw_editor/tests/` or as an inline test in `panel_type.rs` + `tab_viewer/mod.rs`):

```rust
// In tools/aw_editor/src/panel_type.rs tests (or a new tests/panel_registration.rs):

#[test]
fn regional_archetype_panel_registered_in_panel_type_enum() {
    let all = PanelType::all();
    assert!(
        all.contains(&PanelType::RegionalArchetype),
        "PanelType::RegionalArchetype must be in PanelType::all() to surface in \
         the View/Window menu (regression catches F.5-paint.E gap)"
    );
    assert_eq!(PanelType::RegionalArchetype.title(), "Regional Archetypes");
    assert_eq!(PanelType::RegionalArchetype.category(), PanelCategory::Content);
    assert!(!PanelType::RegionalArchetype.description().is_empty());
}

// In tools/aw_editor/src/tab_viewer/mod.rs tests (or a new tests/tab_viewer_panel_registration.rs):

#[test]
fn editor_tab_viewer_instantiates_regional_archetype_panel() {
    use crate::panels::Panel;
    let tab_viewer = EditorTabViewer::new();
    assert_eq!(tab_viewer.regional_archetype_panel.name(), "Regional Archetypes");
}
```

These two tests together would have caught F.5-paint's gap immediately:

- The first fails if the `PanelType::RegionalArchetype` variant is missing.
- The second fails if the field or initializer is missing on `EditorTabViewer`.

**Pattern B (full editor instantiation through to the menu_bar render) is deferred** — it would require a headless egui context and a way to capture the rendered View/Window menu items, which is non-trivial in the editor's current test harness. The combined Pattern A + light-Pattern-B above is sufficient regression coverage; Pattern B can be revisited if F.5-overlay-and-gate or a future panel addition needs deeper menu-rendering verification.

### §5.3 — Forward-pattern: catch struct-without-PanelType-variant drift

The deeper architectural gap is that **no test enforces "every panel struct in `tools/aw_editor/src/panels/` has a corresponding `PanelType` variant"**. The current test suite (e.g., [`test_panel_type_all` at panel_type.rs:593](../../tools/aw_editor/src/panel_type.rs#L593)) only asserts a minimum count and presence of specific known variants. A future panel addition could repeat F.5-paint's mistake.

**Recommended (out of F.5-paint.F-fix scope; flagged for a separate hardening pass)**: add a workspace-level test that uses Cargo metadata + glob to enumerate `panels/*.rs` modules and assert each one's `Panel`-trait struct has a matching `PanelType` variant. This is a workspace-hardening task, not a remediation task — track it separately.

### §5.4 — Andrew-gate verification post-fix

After F.5-paint.F-fix lands, Andrew's verification procedure is:

1. Open the editor at the F.5-paint.F-fix commit.
2. Open the View → "Panels" submenu. Verify "Regional Archetypes" appears in the list.
3. Click "Regional Archetypes" to add the panel to the dock. Verify the panel renders (sliders + palette + Save/Load buttons visible).
4. Verify brush size / falloff / palette controls respond to interaction (slider drag changes value; archetype dropdown changes selection).
5. **Save/Load button verification deferred** to F.5-paint.G-saveload-diagnostic + F.5-paint.H-saveload-fix per §7.1 below.
6. Brush UX verification deferred similarly to a post-F-fix re-verification gate (since brush UX needs the panel reachable to be testable).

---

## §6 — Test methodology recommendation

Per §5.2: combined Pattern A + light-Pattern-B test in two parts:

- A `panel_type.rs` test asserting `PanelType::RegionalArchetype` exists with correct title + category + non-empty description.
- A `tab_viewer/mod.rs` test asserting `EditorTabViewer::new()` instantiates the `regional_archetype_panel` field with the correct `name()`.

Code-shape sketches in §5.2. Both tests should land in F.5-paint.F-fix.

Pattern B (full editor instantiation through to the menu_bar render) is deferred — out of remediation scope, requires headless egui infrastructure that doesn't yet exist in the editor's test harness.

---

## §7 — Out-of-scope observations

### §7.1 — Save/Load silent failure (deferred to F.5-paint.G-saveload-diagnostic)

Andrew observed "Save Mask" / "Load Mask" buttons producing no visible effect. This observation is **misdirected**: the buttons Andrew clicked were on the existing Terrain panel (not the F.5-paint panel, which was unreachable). The F.5-paint panel's Save / Save As / Load / Clear buttons (in `show_persistence_section` at [regional_archetype_panel.rs:516](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L516)) wire to `rfd::FileDialog` calls and the panel's owned save/load API ([regional_archetype_panel.rs:213-281](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L213) — `save_mask_to`, `load_mask_from`, plus internal `trigger_save_dialog` / `trigger_save_as_dialog` / `trigger_load_dialog`).

Once F.5-paint.F-fix makes the panel reachable, Andrew can re-verify save/load. **If save/load surfaces fresh issues post-fix**, those become the trigger for F.5-paint.G-saveload-diagnostic (a separate session per Andrew's Q3 resolution).

Note that the F.5-paint.C save/load tests at [regional_archetype_panel.rs:1030-1180](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L1030) verify the panel's save/load logic in isolation (round-trip byte-identity, error path on no-mask save, falloff-radius-pixels inheritance on load, sibling-directory layout helpers). The logic itself is exercised; what was untested was whether it could be reached through the editor UI.

### §7.2 — Brush UX paint-without-visible-change (deferred until panel reachable)

Andrew's "click+drag registers as undo entries on the existing Terrain panel" observation is also misdirected: Andrew was testing the Terrain panel's sculpt brush, not the unregistered F.5-paint panel's brush. Once F.5-paint.F-fix makes the F.5-paint panel reachable, brush UX gets re-verified at that gate.

The F.5-paint panel's brush logic is at [regional_archetype_panel.rs:130-279](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L130) (`queue_paint_op`, `apply_pending_paint_ops`, `paint_circle`, `screen_to_world_xz_y0`). The 10 brush tests at [regional_archetype_panel.rs:828-1015](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L828) verify brush behavior in isolation. Whether the brush produces visible feedback in the actual editor depends on whether F.5-paint.F-fix wires the panel's pointer events to the viewport (the F.5-paint prompt §2.2 mentioned this, but F.5-paint.B's actual implementation only added the brush-queue + apply API — there is **no viewport-pointer-event integration code in F.5-paint**, which means even after panel registration is fixed, the panel will render but pointer events on the viewport won't reach `queue_paint_op`).

This is **strictly an additional gap noted for F.5-paint.F-fix or a follow-up session**; it is NOT investigated further in F.5-paint.E-diagnostic per the prompt's anti-drift discipline. F.5-paint.F-fix's prompt drafting should decide whether viewport-pointer-event wiring is in scope or deferred to another session. Recommendation: defer to a separate session if the wiring touches non-trivial viewport state; bundle with F-fix if it's a few lines.

### §7.3 — F.4.G Andrew-gate verdict not affected

F.4 is unaffected by F.5-paint's regression. F.4.G's pending Andrew-gate covers the runtime data path (`WorldGenerator.regional_archetype_mask` + chunk generation through painted regions); that runs independently of editor UI reachability. F.5-paint.E-diagnostic does not touch F.4's status.

### §7.4 — Methodological lesson (worth preserving in §10 deviations log)

F.5-paint.A's panel registration was specified in the F.5-paint prompt §2.1 step 7 ("Panel registration with editor: per §1.2 findings, register the panel with the editor's panel system. Likely a single line addition to `tools/aw_editor/src/panels/mod.rs` or wherever existing panels register."). The prompt explicitly anticipated registration as a step. The author of F.5-paint.A satisfied the literal "single line addition to `tools/aw_editor/src/panels/mod.rs`" but missed the second clause "or wherever existing panels register" — and the 30 unit tests across F.5-paint.A-C exercised the panel struct programmatically (via `RegionalArchetypePanel::default()` and direct method calls) without ever instantiating the path that would have surfaced the gap.

**This is the precise failure mode the campaign §0 lesson application targets**: code-level PASS at struct-shape isn't plan-level PASS at user-facing-deliverable-shape. The Andrew-gate caught the gap; unit tests structurally couldn't because they didn't engage the editor's panel-registry layer.

**Forward implication for F.5-paint.F-fix and future panel additions** (worth landing in the §10 entry):

When adding a new editor panel, the verification must include either:
- **Pattern A** (recommended; landing in F.5-paint.F-fix): a test that asserts the panel appears in `PanelType::all()` AND that `EditorTabViewer::new()` instantiates the panel field.
- **Pattern B** (deferred; future hardening): an integration test that exercises the full editor instantiation path including menu rendering. Out of F.5-paint.F-fix scope.

---

## §8 — Estimated F.5-paint.F-fix session scope

**Code changes**: 10 small edits across 2 files (~20-30 lines net additions). All edits mirror existing canonical patterns from `Blueprint` / `Terrain`. No new dependencies. No feature flags. No conditional compilation.

**Test additions**: 2 unit tests (Pattern A + light-Pattern-B per §5.2). ~25-40 lines of test code total.

**Estimated commit count**: 1-2 commits.
- Option A (single commit): "F.5-paint.F-fix: register RegionalArchetypePanel in editor panel system" — bundles the 10 code edits + 2 tests + closeout doc updates.
- Option B (split): F.5-paint.F-fix.A = code edits + tests; F.5-paint.F-fix.B = campaign doc closeout.
- Recommendation: **single commit (Option A)** — the changes are mechanical, mirror existing patterns, and don't carry independent revertable value. The campaign doc closeout can land in the same commit since no hash-fixup is needed (closeout commit references its own predecessor sub-phase commits, not its own hash).

**Estimated wall-clock**: 30-60 minutes.

**Andrew-gate scope**: same checklist as the original F.5-paint Andrew-gate, narrowed to the panel-reachability dimension only. Andrew opens View/Window menu, confirms "Regional Archetypes" appears, clicks to add it, confirms the panel renders + sliders respond. Save/Load and brush UX deferred to subsequent gates per §7.1 / §7.2.

**Out of F.5-paint.F-fix scope** (will need separate sessions):

- Save/Load functional verification (F.5-paint.G-saveload-diagnostic + F.5-paint.H-saveload-fix per Q3 resolution). Wait until panel is reachable to test.
- Viewport-pointer-event wiring for brush placement (per §7.2). Decide in F.5-paint.F-fix prompt drafting whether to bundle or defer.
- Workspace-level test for "every panel struct has a PanelType variant" (per §5.3). Out of remediation scope; track as workspace hardening.
- Climate Preview overlay + integration tests + 5-region Andrew-gate (still F.5-overlay-and-gate's scope). Comes after F.5-paint.F-fix + any subsequent save/load + brush remediation lands.

---

## §9 — Summary

**Root cause**: F.5-paint.A added the panel module declaration (`pub mod regional_archetype_panel;`) but did not modify the editor's `PanelType` enum or `EditorTabViewer` struct. Without a `PanelType::RegionalArchetype` variant, the panel cannot surface in the View/Window menu, "Add Panel" popup, or console panel-list — those three surfaces all iterate `PanelType::all()` as their source of truth. The panel struct exists and compiles, has 30 passing unit tests, but is never instantiated by the editor's UI construction code.

**Fix shape**: 10 mechanical edits across `panel_type.rs` (7 surfaces) and `tab_viewer/mod.rs` (3 surfaces) plus 2 regression tests. Mirrors `Blueprint`'s existing canonical pattern. ~30-60 minutes wall-clock.

**Lesson**: unit-test PASS at struct shape ≠ Andrew-gate PASS at user-facing deliverable shape. Pattern A test lands in F.5-paint.F-fix to catch this regression class permanently.

**Out of scope** (for later sessions): save/load functional verification, brush viewport-pointer wiring, workspace-level "every panel struct has a PanelType variant" hardening, and the F.5-overlay-and-gate session that closes F.5 as a whole.
