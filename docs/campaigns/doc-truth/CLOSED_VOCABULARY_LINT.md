# Closed-Vocabulary Lint Spec — Documentation Truth campaign

**What this is.** Two closed vocabularies whose members are, by campaign decision, either *forbidden as a present-tense AstraWeave claim* (Vocab A poison strings) or *forbidden as provenance-free self-praise* (Vocab B superlatives). Unlike open-ended capability prose — which only the inventory sweep can audit — these are **closed sets**, so `git grep` is exhaustive where sampling is lossy (the D.1.A failure: it corrected `CLAUDE_MD_HARDENING:296` but missed the sibling at `:319`, and `COMPREHENSIVE:13` but missed `:1022`). This file is the authoritative spec the **D.3 regression lint** consumes; the evidence list of current occurrences is [`CLOSED_VOCABULARY_OCCURRENCES.md`](CLOSED_VOCABULARY_OCCURRENCES.md).

**Lint scope.** All tracked prose: `*.md` plus `gh-pages/**` (`.md`/`.html`/`_config.yml`), `.zencoder/**/*.md`, `.github/*.md`, per-crate `README.md`. **Excluded** (never linted): `docs/journey/**`, `docs/archive/**` (historical artifacts), `docs/campaigns/doc-truth/**` (this campaign's audit trail legitimately *quotes* the forbidden strings as evidence), `.github/copilot-instructions-old-backup.md`. Source-code comments are out of scope (a separate concern from prose).

**Lint posture (per the framework D.3 plan).** Warn-only for two weeks, then enforce. A match is a *candidate* violation; the keep-vs-act rule below defines the false-positive set the lint must allow (cited competitor figures, literal test params, registry-linked headlines, correctly-dated historical notes).

---

## Vocabulary A — poison strings (retired/fabricated values forbidden as current AstraWeave truth)

Seeded from the `CLAIMS_REGISTRY.md` Retired table + D.0 §4 + D.0.1 §5. Each is a value that is FABRICATED (delete) or STALE/superseded (correct/retire) per `D0_CLAIMS_INVENTORY.md` §1.2 ground truth.

<!-- VOCAB-A-START -->
```
# Retired entity-capacity / superseded counts
103,500 | 103k | 610,000 | 610k
4,907 | 2,560              # fluids tests -> 738 (registry: fluids-test-markers; 2,560 superseded by W.1 deletion 2026-06-20)
27,000+                    # workspace tests -> ~39,900 markers (registry: test-markers-total)
3,892 | 6,100+             # editor tests -> 9,427 markers (registry: editor-test-markers)
82+ crates | 128 members | 128 workspace | 126 members | 49 production | 47 crates | 44+ crates | 55 crates | 59 library crates
                           # -> 141 members at 2026-10-16 after #205 (registry: workspace-members). 130, 133, 139, and 140 are earlier snapshots, not poison.
# Fabricated competitor multipliers / baselines
10.4× | 10.4x | 2.1-5.2× | 2.1-5.2x | 3-6× | 3-6x
Unity 9,900 | Unreal 20k-50k
# Fluids surfaces that were deleted / never existed
ResearchFluidSystem | UnifiedSolver | DFSPH (as live solver) | IISPH (as live solver) | SPH/FLIP (as the solver)
# Dormant-advertised-as-live
99.96%                     # SpatialHash broadphase -> dormant; real broadphase Rapier DefaultBroadPhase
# Wrong runtime LLM default
Hermes 2 Pro | hermes2pro | qwen3:8b | Qwen3-8B   # (as the LIVE default; runtime default is phi3:medium)
# Stale structural counts
7 stages | 7-stage         # -> 8 stages (incl. SYNC)
6 AI modes | 6 planning modes   # -> 7 modes (feature-gated) (registry: ai-modes)
# Phantom cargo aliases (only editor/editor-release/editor-dev exist)
check-all | build-core | test-all | clippy-all | build-working
# Wrong / nonexistent net security + transport
TLS 1.3 | Ed25519          # (as live in-engine net security; real is HMAC-SHA256, no in-engine TLS)
QUIC | quinn               # (as live transport; real is WebSocket / tokio-tungstenite)
# Fabricated type names (do not exist in code)
EntityView | HealthView | ObjectiveHint | HazardHint | MaterialGraph | ShaderTarget | CompileResult | dev_unsigned_assets
# Stale measured metrics — minted by D.2.B (2026-06-27) / Path-B.2 (2026-06-29); see registry
59.3% | 59.3 | 94.57% | 94.57   # coverage -> 57.35% whole-workspace (registry: coverage-weighted); 59.3 = 29-crate subset, 94.57 = v4.2.0 figure debunked by the v5.0.0 audit
977                        # miri tests -> 1,059 (registry: miri-tests; 977 = the dated MIRI_VALIDATION_REPORT.md record)
# (frame-time 2.70 ms / 370 FPS / 84% headroom are deliberately NOT listed — see "Frame-time carve-out" below)
```
<!-- VOCAB-A-END -->

## Vocabulary B — provenance-free superlatives (forbidden as AstraWeave self-description)

<!-- VOCAB-B-START -->
```
world-class | world class
world's first | first AI-native | first ... engine
industry-leading | industry precedent | sets the industry
exceeds industry | exceeds AAA | exceeds Unity | exceeds Bevy | exceeds Unreal
most validated | most comprehensive | most rigorously
rivals Unity | rivals Unreal | matches Unity | matches UE5 | matches/exceeds
parity with AAA | AAA-grade | AAA-class | AAA standards
no other engine | unique in industry | no competitor has this
competitive with UE5 | competitive with Unity
production-grade | production-ready          # status words — see keep-vs-act rule (NOT auto-delete)
```
<!-- VOCAB-B-END -->

---

## Keep-vs-act rule (the false-positive allowlist the lint must honor)

For each occurrence:

1. **Poison string asserting the value as current AstraWeave truth** → ACT per four-action: retired numbers DELETE or retire-to-registry; stale-with-§1.2-arbiter CORRECT (link the registry); aspirational REWRITE.
2. **Superlative describing AstraWeave** → remove the provenance-free adjective *surgically*; keep the factual remainder. "world-class geometry rendering but CPU-bound lighting" → drop "world-class", keep the rest. Deleting a superlative ≠ deleting the sentence.
3. **`production-ready` / `production-grade`** → **KEEP** where the surface is genuinely shipped and has production callers; **REWRITE-to-honest** where dormant / in-design / example-scoped. Not an auto-delete.
4. **KEEP byte-identical** when the occurrence is any of:
   - a **cited competitor figure** with provenance (e.g. the Reddit-sourced Unity DOTS "1 million entities" number; "Fyrox is the production-grade Rust+egui reference" — about a *competitor*, not AstraWeave);
   - a **literal test parameter** (e.g. `676` in `astraweave-ai/tests/planner_tests.rs`);
   - the **registry-homed headline** (`12,700` agents) **provided it links** to `CLAIMS_REGISTRY.md#agents-capacity-60fps` rather than restating a bare comparative;
   - a **correctly-dated historical note** in a campaign/audit doc that scopes itself to a past commit (e.g. F.0 audit findings under a "superseded by F.1" banner);
   - an occurrence inside the **excluded paths** (journey/archive/doc-truth/old-backup).

The union of acted + kept occurrences is the complete closed-vocabulary inventory. The lint flags any *new* occurrence (a string from A or B) that is not in the allowlist of rule 4.

---

## Front-door check (2026-10-16)

`aw_doc_lint` enforces this even in `--mode warn` (the vocabulary scan stays warn-only):

- Both `required` lists in `.github/workflows/determinism-replay.yml` must agree, and `docs/current/ENGINE_STATUS.md` must name each case in the visible text. A name that exists only inside an HTML comment does not count. An unclosed block comment hides through the end of the page.
- `docs/current/ENGINE_STATUS.md` must state the under-11 ms GTX 1660 Ti frame budget, the earlier heavy-scene `3.688 ms`, and the later median frame sample as `3.117 ms` on a measurement sentence, that assets are CC0, that local LLMs propose, and that the deterministic simulation validates. #216 is on main. #371 stays open. #372 is closed. `3.117 ms` and `3.688 ms` have to be their own tokens on a sentence with whole-word `median` or `measured`, or whole-word `sample` with whole-word `is` or `was`. `11 ms` is its own token. `13.688 ms`, `3.688 msec`, and `111 ms` do not match. Naming a `capture` is not a measurement. Whole-word `not`, `no`, `none`, `nobody`, `nothing`, `never`, `cannot`, `neither`, or `nor`, or an `n't` contraction, keeps that sentence off the set. Inline tags and emphasis are stripped before that scan, so `n<b></b>ot`, `n*ot*`, and `n**ot**` read as `not`. `11 ms`, `Local LLMs propose`, and `deterministic simulation validates` are negation-aware the same way. `CC0`, `GTX 1660 Ti`, and `Aravath` have to sit on a sentence that is not negated. The apostrophe may be ASCII, U+2018, U+2019, U+02BC, or U+FF07, including a character reference that decodes to one of those. Before those checks the page folds U+FF01–U+FF5E into ASCII and U+3000 into a space, and drops non-whitespace controls, the Cf and Mn characters `unicode_categories` 0.1.1 classifies, the Unicode 15 Cf and Mn gaps that crate misses, and U+115F, U+1160, U+3164, and U+FFA0. `unmeasured` is not `measured`. Visible text is the pulldown-cmark render, with footnotes on. Code stays as that parser emitted it. A numeric character reference in text is decoded when it ends in a semicolon, including any run of leading zeros. A markdown value in 0x80–0x9F is U+FFFD, not a Windows-1252 character. C0 controls other than tab, line feed, form feed, and carriage return, and DEL, are U+FFFD in markdown text and in raw HTML. The fold keeps U+FFFD. Form feed stays whitespace. A named reference in markdown text still ends in a semicolon, and a second pass does not run, so `&amp;not;` keeps the word `not`. Raw HTML and alt text decode a numeric reference with an optional semicolon: 0 is U+FFFD, and 0x80–0x9F use the Windows-1252 row, so 0x92 and 146 are U+2019 and 0x85 is U+2026. The five undefined slots 0x81, 0x8D, 0x8F, 0x90, and 0x9D are U+FFFD. A referenced footnote definition is scanned. An unreferenced one is omitted. A defined footnote reference writes its digits as superscripts, or U+00B9 when the label has no digit, so the letters around it do not join. An unclosed `<!--` hides through the end of its block, including across a blank line. `<!-->`, `<!--->`, and `--!>` close the comment. A comment inside a code span or a fence stays text. A `script` or `style` body stays. Link labels join the surrounding letters. A link or image title, and an HTML `title` attribute, are scanned on their own line at the end of the enclosing block. A link reference definition is omitted. An autolink keeps its URL. A `br` tag inserts a break. U+1734 stays. A sentence that only names the check does not count.
- `README.md` must not contain a frame figure: a number followed by `ms`, `msec`, `millisecond`, or `milliseconds`, including the fullwidth block U+FF01–U+FF5E (so a fullwidth `ms` matches) and U+3000 as a space, and not by a further letter (`messages`, `msecs`). The scan drops `*`, backticks, paired `__` and `~~`, and a single `_` that is not between two word characters. A single `~` stays, so `3.117_ms` and `3.117~ms` are literal. It keeps quoted and unquoted `alt` text. A `/` or any ASCII whitespace before `alt` starts that attribute, so `<img/alt="3.117 ms">` inside an HTML block counts. A non-comment `<` that never reaches `>` is text. A comment ends at `-->` before quotes, so an apostrophe inside the comment does not hide the next sentence. An unclosed block comment hides through the end of the block, including a blank line. `<!-->`, `<!--->`, and `--!>` are closed. A comment in a code span or a fence stays. Link labels join, so a label `.117` between `3` and ` ms` is a figure. A link or image title, and an HTML `title` attribute, are scanned on their own line at the end of the enclosing block. A reference definition is omitted. A referenced footnote definition is scanned, including a URL followed by a quoted figure. An unreferenced footnote definition is omitted. A `br` tag is a break. U+1734 stays between a number and its unit. A `script` or `style` body is scanned. A tag, including a closing tag with attributes such as `</style data-x>`, ends at `>` outside quotes. A dot in `<script.foo>` does not hide the rest of the file. A backslash immediately before a newline is a hard break, so the newline still joins the number and the unit. An autolink (`http://`, `https://`, `mailto:`) is scanned. Comments and closing tags contribute no text. Code stays as pulldown-cmark emitted it. Text decodes a numeric character reference that ends in a semicolon, including any run of leading zeros, up to six significant hexadecimal digits or seven significant decimal digits. A markdown value in 0x80–0x9F is U+FFFD, not a Windows-1252 character. C0 controls other than tab, line feed, form feed, and carriage return, and DEL, are U+FFFD in markdown text and in raw HTML. The fold keeps U+FFFD. Form feed stays whitespace. Raw HTML and alt text decode a numeric character reference the way the HTML tokenizer does: the semicolon is optional, leading zeros do not count, 0 is U+FFFD, and 0x80–0x9F go through the Windows-1252 row (0x92 and 146 are U+2019, 0x85 is U+2026). The five undefined slots 0x81, 0x8D, 0x8F, 0x90, and 0x9D are U+FFFD. An HTML5 named character reference in raw HTML still ends in a semicolon (a name may include digits; `&amp;nbsp;` is not decoded a second time; a two-character expansion such as `ThickSpace` is kept). Raw HTML and alt text also accept an HTML5 legacy name with no semicolon, including `&nbsp`. In alt text that name is not consumed when the next character is `=` or an ASCII letter or digit. A semicolon-less `&nbsp` in markdown text stays text. It drops the Cf and Mn characters `unicode_categories` 0.1.1 classifies, including U+00AD, U+034F, U+061C, U+180B through U+180E, U+200B through U+200F, U+202A through U+202E, U+2060 through U+2064, U+2066 through U+206F, U+FE00 through U+FE0F, U+FEFF, and U+E0001, plus the Unicode 15 gaps that crate misses (U+08E2 and U+180F among them) and the Default_Ignorable letters U+115F, U+1160, U+3164, and U+FFA0, and other non-whitespace control characters before the unit. Named gaps such as `NegativeThickSpace`, `DotDot`, `tdot`, `TripleDot`, and `DownBreve` are in that set. Whitespace, including a hair space, stays and becomes a normal space. The measured sample stays on the engine-status page.
- `README.md` must have a `## In progress` section listing #334, #366, and #383, and no other issue number. That section must not call the work shipped, landed, or merged. The section runs from the rendered h2 whose folded text is exactly `in progress`, setext or ATX. It ends at the next rendered h2 after that heading is finished, unless the heading's folded text, with every run of Unicode spaces mapped to one ASCII space and trimmed, is `in progress` or empty. A repeated heading, double spaces, a no-break space, an em space, a tab, a form-feed-only heading, a zero-width-only heading, and setext `In<br>progress` do not end it. A fenced or commented `## ` line does not end it, and a heading that only contains those words does not open it. Issue numbers are read from that section after the same fold, so `#<!-- -->999`, `#&#x200b;999`, `#&shy;999`, `#&zwj;999`, U+FF03, and fullwidth digits are `#999`. Those three words are read from the folded render, so a comment, a tag, U+200B, U+200D, U+00AD, or a fullwidth form between the letters still counts. #216, #257, #258, #281, #283, #284, #307, #320, #335, #367, #374, and #377 are on main and are not in that list.
- Player-facing files (`README.md`, `CONTRIBUTING.md`, `docs/README.md`, `docs/QUICKSTART.md`, `docs/current/PROJECT_BRIEF.md`, `docs/guides/**/*.md`) must not use the retired game title. The check reads the rendered visible text after the same fold as the frame gate, so a comment, a tag, U+200B, U+200D, U+00AD, or a fullwidth form between the letters still counts. `veilweaver_` crate names and the archive directory `projects/veilweaver` are allowed.

## Frame-time carve-out (D.3.1) — why `2.70 ms` is NOT in Vocabulary A

The frame-time poison generation (`2.70 ms` / `370 FPS` / `84% headroom`, **as-achieved** — superseded by the measured System **0.965 ms** / mimalloc **0.709 ms** @ 1,000 entities; `CLAIMS_REGISTRY.md#frame-time-1000-entities`) is **deliberately excluded** from the closed-vocabulary lint, per the D.3.0 ratification (Option A).

**Why (it fails the closed-set test).** Unlike coverage/miri — whose superseded values (`59.3`, `94.57`, `977`) never legitimately recur except as a dated/superseded record — the string `2.70 ms` is, in the live corpus, *simultaneously*:

1. the **sanctioned reframing** ("2.70 ms was the Week-8 *target*"), which must keep the string (≈10 sites: README, render README, gh-pages ×3, docs/src/README, MASTER_ROADMAP ×3, MASTER_BENCHMARK ×3);
2. a **labeled target** (`gh-pages/benchmarks.md`);
3. a **dated achievement record** (Week-8 sprint rows across `docs/lessons/**`, `docs/audits/**`);
4. a **false positive** (Vec3-lerp `2.70 ns`, deserialize `2.70 ms` in `.bak`, `num_batch 2700ms`);
5. genuine **present-tense poison**.

A pure string match cannot separate (5) from (1)–(3). Listing it would either need ~30+ allowlist keys (mostly in dated docs → low signal) or chronically false-positive, eroding the warn→enforce credibility. So frame-time is **human-review-only**, not lint-enforced.

**Baseline already established.** The present-tense frame-time survivors were one-time-corrected in D.3.1: `.zencoder/rules/repo.md:84` and `docs/lessons/WHAT_DIDNT.md:21` (the bare "**Current**:" claim). Dated Week-8 records (e.g. `docs/lessons/PERFORMANCE_PATTERNS.md` "Current Status (Week 8)" block at :62–73) were left byte-identical per the per-row principle (correcting a dated record falsifies history).

**Documented alternative (Option B, not adopted).** A standing frame-time guard is possible via a *same-line-context* rule — flag `2.70 ms`/`2.7 ms`/`2.70ms` only on lines lacking `target`/`Week 8`/`Week-8` — at the cost of ~12 allowlist keys for the dated audits/lessons that assert it present-tense without those words. Revisit only if a frame-time regression recurs.

---

## Revision history

| Version | Date | Change |
|---|---|---|
| 0.1 (D.1.B) | 2026-06-13 | Created. Vocab A (poison) + Vocab B (superlative) sets seeded from the registry Retired table + D.0/D.0.1 dispositions; keep-vs-act rule + exclusion scope defined for D.3 lint consumption. Pre-edit occurrence totals: 501 A + 415 B across 202 files (see occurrences doc). |
| 0.2 (D.3.1) | 2026-06-29 | **New poison generation + machine-readable markers.** Added the D.2.B/Path-B.2 measured-metric poison to Vocab A: coverage `59.3`/`59.3%` + `94.57`/`94.57%` (→ 57.35% whole-workspace) and miri `977` (→ 1,059). Added the **Frame-time carve-out** (ratified Option A — `2.70 ms`/`370 FPS` deliberately NOT linted; human-review-only; present-tense survivors one-time-corrected). Wrapped Vocab A / Vocab B in `<!-- VOCAB-A/B-START/END -->` markers (and the allowlist table in `<!-- ALLOWLIST-START/END -->`) so the D.3.2 lint reads this spec as its single source of truth. Companion allowlist additions: 14 keys (supersession-context / version-comparison / debunking-context / dated-record / report-description). Authority: `D3_0_RECON.md`. |
| 0.3 | 2026-10-16 | Front-door check documented. Workspace-count comment now points at 139 members. `130` stays off the poison list. |
| 0.4 | 2026-10-22 | Front-door phrases and replay names are checked on `docs/current/ENGINE_STATUS.md`. The `## In progress` list stays on `README.md`. The later frame sample must be a measurement line. The landing page rejects an HTML frame figure. #372 is closed. |
| 0.5 | 2026-10-25 | The landing-page frame scan decodes character references, tags, and markdown emphasis before `ms` or `milliseconds`. The `3.117 ms` sample is a whole token on a measurement sentence. |
| 0.6 | 2026-10-25 | The frame scan also drops `_` and `~`, keeps `alt` text, leaves a non-tag `<` in the text, accepts long leading zeros and the named gaps `NonBreakingSpace`, `ThinSpace`, `zwnj`, `zwj`, `lrm`, `rlm`, `ZeroWidthSpace`, `NewLine`, and `Tab`, drops zero-width characters, and treats `msec` and `millisecond` as units. The `3.117 ms` sentence needs whole-word `median`, `measured`, or `capture`, or `sample` with `is` or `was`, and a negation does not count. |
| 0.7 | 2026-10-27 | `3.688 ms` is a measurement token and `11 ms` is a token, so `13.688 ms`, `3.688 msec`, and `111 ms` do not match. `capture` is not a measurement. `no`, `nobody`, and `nothing` are negations, and `n't` includes `&apos;`, `&rsquo;`, `&#39;`, `&#8217;`, and U+02BC. A closed HTML comment does not supply a phrase or a frame figure, and a comment ends at `-->` before quotes. The frame scan keeps a single `~` and a `_` between word characters, reads unquoted `alt` and autolink text, leaves an unclosed `<` in the text, maps fullwidth digits and U+FF0E, and drops the named spaces and invisible characters listed above. |
| 0.8 | 2026-10-28 | Status text decodes the HTML5 named character reference table and every numeric reference, folds U+FF01–U+FF5E, and drops every Cf and Mn character, so an encoded `not` or `n't` still negates a measurement. `neither` and `nor` are negations. An unclosed `<!--` and a `script` or `style` element are hidden through their end. `CC0`, `GTX 1660 Ti`, and `Aravath` have to sit on a sentence that is not negated. The frame scan uses the same decode, fold, and hidden-markup rules, and a backslash before a newline is a hard break. A non-comment `<` that never reaches `>` stays text. |
| 0.9 | 2026-10-28 | Only a closed HTML comment is hidden. A `script` or `style` body is scanned, a closing tag may carry attributes, and `<script.foo>` does not hide the rest of the file. An unclosed `<!--` stays in the text. Inline tags and emphasis are stripped before the word scan. `none` is a negation. `11 ms` and the two prose phrases are negation-aware. Replay names are matched on the visible text. The invisible set is `unicode_categories` 0.1.1 plus the 20 Cf and 419 Mn characters it misses, and U+115F, U+1160, U+3164, and U+FFA0. The apostrophe includes U+2018. |
| 0.10 | 2026-10-28 | Visible text comes from pulldown-cmark 0.13.5 (tables, strikethrough, and task lists; smart punctuation off). An unclosed HTML comment hides through the end of its block, including a blank line. A comment in a code span or a fence stays. `<!-->` and `<!--->` close. Link labels join the letters around them. Reference definitions are omitted. Autolink text stays. A `br` tag inserts a break. U+1734 stays, because Unicode 15 classifies it as Mc. |
| 0.11 | 2026-10-28 | Character references are decoded only in raw HTML, and only with a semicolon. Text and code stay as pulldown-cmark emitted them. Footnotes are on: a referenced definition is scanned and an unreferenced one is omitted. The retired title and the in-progress words are read from that render. |
| 0.12 | 2026-10-28 | Text decodes a numeric character reference with any run of leading zeros. Code keeps that reference. Raw HTML and alt text accept the HTML5 legacy names with no semicolon. The in-progress section ends at the next rendered heading, and its issue numbers come from that render. The retired title and the in-progress words use the frame fold. A defined footnote reference writes a superscript digit. |
| 0.13 | 2026-10-28 | The in-progress heading is the rendered h2 whose folded text is exactly `in progress`, setext or ATX. Issue numbers in that section are read after the same fold. Raw HTML and alt text decode a numeric character reference with an optional semicolon: 0 is U+FFFD, and 0x80–0x9F use the Windows-1252 row. Markdown text still requires the semicolon. Code keeps the reference. |
| 0.14 | 2026-10-28 | A `/` or any ASCII whitespace before `alt` starts that attribute. A markdown numeric reference in 0x80–0x9F is U+FFFD. The five undefined Windows-1252 slots are U+FFFD. The in-progress section finishes the next h2 before ending, and continues when that heading collapses to `in progress` or is empty. |
| 0.15 | 2026-10-28 | C0 controls other than tab, line feed, form feed, and carriage return, and DEL, are U+FFFD in markdown text and in raw HTML. The fold keeps U+FFFD. Form feed stays whitespace, so it still joins a frame figure and a form-feed-only heading stays empty. |
| 0.16 | 2026-10-28 | A raw scalar in an HTML block and in alt text uses that same replacement, including a raw C1 scalar. A numeric reference in 0x80–0x9F still uses the Windows-1252 row. |
| 0.17 | 2026-10-28 | `--!>` closes an HTML comment. A link or image title, and an HTML `title` attribute, are scanned. |
| 0.18 | 2026-10-28 | A link or image title, and an HTML `title` attribute, are written on their own line at the end of the enclosing block. Alt text stays where the tag sits. |
| 0.19 | 2026-10-29 | The front-door internals restated on `docs/current/ENGINE_STATUS.md` stay in this section. A named reference in markdown text is not decoded a second time, so `&amp;not;` keeps `not`. A footnote reference writes the label's digits as superscripts, or U+00B9 when the label has no digit. |
