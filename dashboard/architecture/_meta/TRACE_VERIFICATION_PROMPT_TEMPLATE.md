# Trace Verification Prompt Template

You are running a verification pass against an architecture trace document for the
AstraWeave engine. The document at {{TRACE_DOC_PATH}} contains markers indicating
unverified or inferred claims:

- [NEEDS VERIFICATION] — claims I was unable to confirm and need checked
- [INFERRED] — claims based on context/filename but not confirmed against code
- [Reasoning not recovered from available sources] — historical rationale not found
- [NEEDS VERIFICATION — <specific scope>] — narrower verification scopes

Your job: read the trace doc, then for each marker, investigate the codebase
and either confirm the claim (removing the marker and stamping the change) or
correct it. You may also add new markers if you discover claims that are stated
confidently but are actually incorrect.

{{OPTIONAL_CONTEXT}}

## Required reading before you start

1. The trace doc at {{TRACE_DOC_PATH}} — read it end to end first.
2. The architecture trace template at
   `docs/architecture/_meta/ARCHITECTURE_TRACE_TEMPLATE.md`.
3. The top-level `CLAUDE.md` for engine-wide context.

Do not begin verifying until you've read all three. You need to understand both
the doc's structure and the engine's conventions before checking individual claims.

## Verification methodology

For each marker, take the following investigation steps depending on marker type.

### Marker: [INFERRED] on a file's role

Investigation steps:
1. View the file directly.
2. Identify its exported types, functions, traits, and public API.
3. Search the codebase for callers/importers of this file.
4. Read 2-3 of the most significant call sites to confirm the role described in
   the trace doc matches actual usage.

Update rule:
- If the inferred role is correct: remove [INFERRED] marker, add a brief citation
  of the evidence (e.g. "Verified — exports XyzManager, called from `foo.rs:42`").
- If the inferred role is partially correct: rewrite the role description to match
  reality, remove [INFERRED].
- If the inferred role is wrong: rewrite the role description, remove [INFERRED],
  and add a note that the previous description was incorrect.
- If still unclear after investigation: leave [INFERRED] in place, add a note
  describing what you investigated and why it remains uncertain.

### Marker: [NEEDS VERIFICATION] on file status (Active/Transitional/Deprecated)

Investigation steps:
1. View the file itself. Check for deprecation markers, TODO comments, or
   explicit status notes in the code.
2. Search the codebase for `use` statements importing from this file.
3. Search for direct path references (e.g. `mod texture_splatting`).
4. Check whether the file has tests in `tests/` directories.
5. If git log is available, check recent modification history. Files untouched
   for many months with no callers are stronger candidates for Deprecated status.
6. Make a determination:
   - **Active**: Imported and used by current code paths
   - **Transitional**: Imported but documented as being phased out, or in a clearly
     reduced-fidelity role (e.g. fallback paths)
   - **Deprecated**: Not imported anywhere; candidate for deletion
   - **Dead**: Confirmed not used; safe to remove

Update rule:
- If you can confidently determine status: update the file's row in Section 5 with
  the verified status and a brief citation (e.g. "Verified Transitional — imported
  only by `terrain.rs:14`, which is itself Transitional").
- If still unclear: leave [NEEDS VERIFICATION] in place and add a note describing
  what you found and what additional evidence would resolve it.

### Marker: [NEEDS VERIFICATION] on an invariant

Investigation steps:
1. Read the relevant code path that should uphold the invariant.
2. Search for `assert!`, `debug_assert!`, `panic!`, or explicit error returns
   related to the invariant.
3. Search for tests that exercise the invariant.
4. Determine whether the invariant is:
   - Enforced by code (asserts/checks present)
   - Enforced by tests
   - Doc-only (no automated enforcement)
   - Not actually held by the current code (broken invariant — this is a finding)

Update rule:
- Update the "Enforced by" column with verified information.
- If the invariant is not actually held by current code, flag this prominently
  in the report (this is a real issue, not just a doc question).

### Marker: [NEEDS VERIFICATION] on testing/validation

Investigation steps:
1. Look for `#[test]` attributes in the relevant source files.
2. Check `tests/` directories at crate and workspace level.
3. Look for `benches/` directories and benchmark files.
4. Check for mutation testing config (`.cargo-mutants.toml` or similar).
5. Check for Miri test markers if relevant to unsafe code.

Update rule:
- Replace [NEEDS VERIFICATION] with concrete findings: number of tests, file
  locations, any notable gaps.

### Marker: [Reasoning not recovered from available sources]

Investigation steps:
1. Check `git log` for commits touching the relevant code (if available).
2. Look at commit messages for context.
3. Search `docs/` for design docs or audit docs covering the decision.
4. Search code comments for inline rationale.

Update rule:
- If reasoning is found: replace the marker with the recovered context, citing
  the source (commit, doc, or comment).
- If still not recovered: leave the marker as-is. It is honest to say the
  reasoning isn't documented.

## Conservative defaults

These defaults exist because the failure mode "agent confidently removes a marker
on weak evidence" is worse than "agent leaves a marker that could have been resolved."

### Default 1: Preserve uncertainty when evidence is thin

If your investigation produces only suggestive evidence (one call site, one comment,
one inference), preserve the marker and note what you found. A marker is a signal
to future readers that an area is worth careful attention. Removing it falsely
makes the doc less trustworthy, not more.

### Default 2: Add markers when you find confidently-stated but unverified claims

If you read the doc and find a claim stated confidently that isn't backed by
evidence, ADD a [NEEDS VERIFICATION] marker. The verification pass is bidirectional:
remove markers where evidence is strong, add markers where evidence is weak.

### Default 3: Do not resolve Open Questions (Section 11)

The items in Section 11 are parked decisions Andrew makes, not facts to verify.
If your investigation produces new information relevant to an Open Question, add
it as a note under that question — but do NOT close out or remove Open Questions.
Even if you think you know the answer, that call is Andrew's, not yours.

### Default 4: Do not restructure the document

Verification updates specific claims within the existing structure. You may:
- Update a row in a table
- Replace a marker with verified text
- Add a brief citation or note
- Add a new marker where you discover one is missing

You may NOT:
- Add or remove sections
- Reorganize the table of contents
- Move content between sections
- Rewrite the executive summary or appendices (unless a specific factual claim
  in them needs correction)

If you believe the document's structure should change, report that as a
recommendation at the end of your verification report — do not act on it.

### Default 5: No editorial additions

This is a verification pass, not an architecture review. Do not add commentary
about whether the architecture is good, whether things should be refactored, or
how the system could be improved. Stick to facts about what exists.

## Update rules

When you update the file:

1. Make targeted edits, not wholesale rewrites of sections.
2. After each marker is resolved, add a brief citation showing how it was verified.
   Format: a parenthetical note or footnote with file:line references.
3. When you remove a marker, the surrounding text should make sense without it
   (don't leave dangling phrases like "this is the case [INFERRED]" with the
   marker stripped but the rest unchanged).
4. After all updates are made, update the metadata block at the top of the doc:
   - Bump the document version (e.g. 1.0 → 1.1)
   - Update "Last verified against commit" to the current commit short hash
   - Update "Last verified date" to today's date

## Reporting requirements

After completing the verification pass, produce a report with the following
structure:

### Summary
- Number of markers investigated
- Number resolved (with marker removed)
- Number left in place (with notes added)
- Number of new markers added
- Number of factual corrections made
- Any findings flagged as urgent (e.g. broken invariants)

### Resolved markers
For each marker you resolved: original claim, evidence found, updated text.

### Markers preserved
For each marker you left in place: what you investigated, what would resolve it.

### Newly added markers
For each marker you added: what claim was previously stated, why it needs
verification.

### Urgent findings
Anything that's not a documentation issue but a real problem (broken invariants,
files that should be Deprecated but are marked Active and being depended on, etc.).

### Recommendations (if any)
Structural recommendations you have for the doc — separate from the verification
work itself.

## Anti-overreach reminder

Your job is verification, not architecture. You read code to check claims. You do
not propose changes to the code. You do not propose refactors. You do not
editorialize about quality. If you have observations beyond verification, put
them in the "Recommendations" section of your report — never in the doc itself.

## Self-check before finishing

1. For every marker I removed, did I cite specific evidence (file paths, line
   numbers, function names)?
2. Where evidence was thin, did I preserve the marker instead of removing it?
3. Did I add markers where I found confidently-stated but unverified claims?
4. Did I avoid resolving any Open Questions?
5. Did I avoid restructuring the document?
6. Did I avoid adding editorial content or recommendations to the doc body?
7. Is the metadata block at the top updated with new version, commit, and date?
8. Does my report cleanly enumerate what was resolved, preserved, added, and
   flagged?

If any of these are "no," fix before submitting.

Now: read the template, read the trace doc at {{TRACE_DOC_PATH}}, read CLAUDE.md,
then begin verification.
