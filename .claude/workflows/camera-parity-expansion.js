// Reusable command: Camera parity-harness expansion.
//
// Blessed on the Unified Camera campaign's C.8 sub-phase (the first dynamic-
// workflow sub-phase). It fans out across camera fixture families, adversarially
// refutes each generated baseline with a HARDENED skeptic protocol BEFORE landing,
// then lands the survivors and runs the suite + non-regression gates. It produces
// fixtures for human review and DOES NOT commit (that is part of its contract).
//
// HONEST RELIABILITY PROVENANCE (do not launder this):
//   - The verifier-reliability finding is DIRECTIONAL, NOT CONTROLLED. It was
//     established on a single campaign (C.8). The first refute run had a ~14%
//     skeptic-null-rate; a hardened re-verification found it TRANSIENT — but this
//     was NOT proven robust across a controlled A/B.
//   - The retry + minimal-fallback skeptic protocol (hardenedSkeptic below) is
//     LOAD-BEARING INSURANCE, not decoration. It is what makes the verifier
//     reliable under concurrency load. The C.8 run itself used the un-hardened
//     3-skeptic verify (which produced the ~14% nulls); this saved command folds
//     in the hardening developed in the C.8 reliability re-run. KEEP IT.
//   - The SHARED context below is a SNAPSHOT of the camera API at C.8 close.
//     File:line citations WILL drift. Agents must re-verify against current
//     source; the citations are a starting map, not ground truth.

export const meta = {
  name: 'camera-parity-expansion',
  description: 'Camera parity-harness expansion: fan out across fixture families, adversarially refute each baseline (HARDENED skeptic protocol) before landing, then land + run the suite. Produces fixtures for human review — does NOT commit.',
  whenToUse: 'Reuse for camera-parity fixture work (a new producer, a new convention, a new hazard family). Blessed on the Unified Camera C.8 run. RELIABILITY IS DIRECTIONAL, NOT CONTROLLED — single campaign; a ~14% first-run skeptic-null-rate found transient on a hardened re-run, not proven across a controlled A/B. The retry + minimal-fallback skeptic hardening is load-bearing insurance under concurrency, not decoration — keep it. The command does NOT commit; human review + commit is part of its contract.',
  phases: [
    { title: 'Generate', detail: 'one agent per fixture family — concrete cases + derived baselines + test code' },
    { title: 'Verify', detail: 'per-fixture adversarial refute, HARDENED: 3 perspective-diverse lenses, each 2 rich-schema + 2 minimal-fallback attempts before terminal null' },
    { title: 'Land & Run', detail: 'land surviving fixtures, run full parity suite + non-regression gates, report (NO commit)' },
  ],
};

const SHARED = `
# Camera parity-harness expansion — shared context (SNAPSHOT; re-verify citations)

You are one agent in a deterministic workflow expanding a camera parity harness.
The canonical architecture (Unified Camera campaign, C.0–C.8):
RenderView is the sole camera-upload payload; Renderer::update_view is the sole
upload entry point; FreeFly (astraweave-camera) and OrbitCamera (tools/aw_editor)
are the two CameraProducer implementations; cinematics flow
CameraKey -> Renderer::tick_cinematics -> apply_camera_key (sanitizes) -> FreeFly
-> RenderView -> update_view.

## LOAD-BEARING SCOPING FINDING (re-confirm, do not re-litigate the conclusion)
The existing parity test compares RENDERED GPU OUTPUT (SHA-256 of LDR bytes), not
RenderView matrices. A SHA of GPU output CANNOT be independently derived from camera
math, and the anti-fabrication constraint forbids a baseline that can only be
produced by running the code and trusting the output. THEREFORE every fixture is a
RenderView/matrix-level test whose baseline is DERIVED from camera math: glam
Mat4::look_to_rh / look_at_rh / perspective_rh and the producer's dir(yaw,pitch)
spherical formula, in the proven style of orbit_camera_producer.rs /
picking_consistency.rs. Land fixtures in the parity harness test file as new
#[test] fns alongside the existing GPU test. The three matrix families need NO GPU.
The cinematics family is GPU-gated (apply_camera_key is private; tick_cinematics —
its only public reach — needs a live Renderer); its comparison is still
RenderView-level (tick output vs an independent look_at_rh baseline).

## CONFIRMED API MAP (SNAPSHOT at C.8 close — RE-VERIFY against current source)
- astraweave_camera::FreeFly { position, yaw, pitch, fovy, aspect, znear, zfar } (no Default).
  view_matrix=look_to_rh(position, dir(yaw,pitch), Y); view_matrix_camera_relative=look_to_rh(ZERO,dir,Y);
  proj_matrix=perspective_rh(fovy, aspect.max(0.01), znear, zfar); dir(yaw,pitch)=Vec3(cy*cp,sp,sy*cp).normalize();
  to_render_view() (trait, world); to_render_view_camera_relative() (concrete); sanitize() clamps fovy∈[10°,170°], znear≥0.0001, zfar≥znear+0.001, aspect≥0.01.
- astraweave_camera::Projection::perspective(fovy,aspect,znear,zfar): matrix=perspective_rh(fovy, aspect.max(0.01), znear, zfar); stores PRE-floor aspect; debug_assert znear>0, zfar>znear.
- astraweave_camera::RenderView { view, projection, view_proj, inverse_view, inverse_view_proj, position, view_dir, fovy, aspect, znear, zfar } (PartialEq). RenderView::new(view, &Projection, position, view_dir).
- aw_editor_lib::viewport::OrbitCamera::new(focal, distance, yaw, pitch); position()=focal+spherical; view_matrix=look_at_rh(position(),focal,Y); view_matrix_relative=look_at_rh(ZERO,-eye_offset,Y); projection_matrix=perspective_rh(fovy, RAW aspect, near, far) [NO floor — diverges from to_render_view's Projection-floored path below the floor]; set_aspect(w,h) only if h>0; set_fov(deg)/fov_degrees()/fovy().
- astraweave_cinematics::CameraKey { t, pos:(f32,f32,f32), look_at:(f32,f32,f32), fov_deg }; ::new, ::at_origin; sanitize() clamps fov_deg∈[10,170] and look_at==pos -> (pos.0+1,pos.1,pos.2); lerp(&other,t). Timeline/Track/Time/Sequencer; Sequencer emits a CameraKey when from < k.t.0 <= to; step errors if next_t > duration+0.001.
- apply_camera_key(cam:&mut FreeFly, k:&CameraKey) PRIVATE: clone+sanitize; dir=(look-pos).normalize_or_zero(); yaw=dir.z.atan2(dir.x); pitch=dir.y.clamp(-1,1).asin(); cam.position/yaw/pitch set; cam.fovy=fov_deg.to_radians(); aspect/znear/zfar persist. Renderer::tick_cinematics(&mut self, dt, &mut FreeFly) PUBLIC: needs load_timeline + play_timeline; requires a live Renderer (GPU).
- f32 note: cos(FRAC_PI_2 as f32) ≈ -4.37e-8 (NONZERO) — FreeFly stays finite at exact ±pi/2; OrbitCamera's target-based look_at_rh CAN reach exactly parallel and NaN. VERIFY by evaluating, not by assuming.

## GLOBAL CONSTRAINTS (bind EVERY agent — there is no mid-run human escalation)
1. NO agent modifies source outside the parity-harness test file. No producer/renderer/cinematics source.
2. NO agent modifies the canonical types/methods (RenderView, FreeFly, OrbitCamera, CameraKey, sanitize, apply_camera_key, tick_cinematics, update_view) — they are the system under test.
3. NO agent invents a baseline it cannot independently justify. Derive from camera math (state the formula) or capture canonical-producer output (state how). Unverifiable -> report, do not land.
4. NO agent applies byte-equivalence where expected-divergence is correct (large-world camera-relative vs world-relative; OrbitCamera below-floor aspect seam).
5. NO agent fabricates a fixture to avoid reporting a gap. Untestable hazard -> gap_report.
6. NO agent commits. The workflow stops at the report for human review + commit.
7. Cinematics fixtures use the canonical path ONLY (CameraKey -> tick_cinematics -> apply_camera_key -> FreeFly -> RenderView).

## STYLE
Match orbit_camera_producer.rs / picking_consistency.rs: small fixture helpers, focused #[test] fns,
rich assert messages, matrix epsilon ~1e-5/1e-6, relative tolerance for large magnitudes ((mag*1e-4).max(1e-3)),
f64 (DMat4/DVec3) references for precision cross-checks. Compare matrices entry-wise (no exact-eq on derived values).
`;

const FAMILIES = [
  { key: 'extreme_pitch', title: 'Extreme pitch', brief: `HAZARD: pitch singularities near ±π/2. FreeFly.dir stays finite at exact f32 ±π/2 (residue); OrbitCamera's target-based look_at_rh can degenerate. OWN the producer-side pitch singularity (FreeFly + OrbitCamera, NO GPU). Cases: pitch just inside ±clamp; pitch exactly ±π/2 (assert the producer's ACTUAL behaviour, finite or NaN — verify, don't assume); OrbitCamera at max_pitch. BAR: match-derived-baseline-within-epsilon. Skeptic rejects pitch not near ±π/2.` },
  { key: 'non_square_aspect', title: 'Non-square aspect', brief: `HAZARD: aspect handling, vertical-FOV invariance, the .max(0.01) floor (matrix floored, FIELD raw), and OrbitCamera::projection_matrix (raw) vs to_render_view (floored) divergence below the floor. NO GPU. Cases: ultrawide 21:9; portrait 9:16; degenerate-narrow <0.01 (floor discipline); optional orbit raw-vs-floored seam (expected-divergence). BAR: match-derived-baseline-within-epsilon (and expected-divergence for the below-floor seam).` },
  { key: 'large_world_positions', title: 'Large world positions', brief: `HAZARD: f32 precision at 1e5–1e7, and the DESIGNED divergence of world-relative vs camera-relative variants. BAR CRITICAL: do NOT assert the two variants byte-equal (constraint 4). Assert: rotation agree (FreeFly) / camera-relative rotation matches f64 truth while world drifts (OrbitCamera); camera-relative translation ~0, world large; position preserved in both; each variant vs its OWN derived/f64 baseline. Use f64 (DMat4/DVec3) references. NO GPU.` },
  { key: 'cinematics_driven', title: 'Cinematics-driven', brief: `HAZARD: the newest path. Verify CameraKey -> tick_cinematics -> apply_camera_key -> FreeFly -> RenderView equals the geometric intent. REQUIRES GPU (apply_camera_key private; tick_cinematics needs a Renderer — reuse the harness acquire_device + Renderer::new_from_device). Cases: normal keyframe; sanitize clamps out-of-range fov; sanitize resolves degenerate look_at==pos -> +X; lerped mid-keyframe. BAR: match the INDEPENDENT target-based look_at_rh baseline within epsilon (NOT a re-implementation of apply_camera_key's atan2/asin round-trip). If GPU unavailable the land step reports the skip; still generate.` },
];

const FAMILY_OUTPUT = {
  type: 'object',
  required: ['family', 'equivalence_target_confirmation', 'gap_report', 'cases'],
  properties: {
    family: { type: 'string' },
    equivalence_target_confirmation: { type: 'string', description: 'Confirm this family is RenderView/matrix-level testable (and whether it needs GPU). Gaps go in gap_report.' },
    gap_report: { type: ['string', 'null'], description: 'Non-null if a hazard is not meaningfully testable against the harness. Null if fully covered.' },
    cases: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'description', 'config', 'hazard_targeting', 'equivalence_bar', 'baseline', 'derivation_method', 'requires_gpu', 'test_code'],
        properties: {
          id: { type: 'string' }, description: { type: 'string' }, config: { type: 'string' },
          hazard_targeting: { type: 'string' }, equivalence_bar: { type: 'string' },
          baseline: { type: 'string' }, derivation_method: { type: 'string' },
          requires_gpu: { type: 'boolean' }, test_code: { type: 'string' },
        },
      },
    },
  },
};

// HARDENED skeptic schemas (the load-bearing part — keep both).
const SKEPTIC_SCHEMA = {
  type: 'object',
  required: ['lens', 'verdict', 'confidence', 'independent_derivation', 'independent_value', 'reasoning'],
  properties: {
    lens: { type: 'string', enum: ['math-derivation', 'hazard-targeting', 'equivalence-bar'] },
    verdict: { type: 'string', enum: ['refuted', 'refutation_failed'] },
    confidence: { type: 'string', enum: ['low', 'medium', 'high'] },
    independent_derivation: { type: 'string', description: 'The INDEPENDENT computation (formulas + steps), by a DIFFERENT method than the fixture. A verdict with no shown derivation is a FAILURE, not a pass.' },
    independent_value: { type: 'string', description: 'The CONCRETE value(s): matrix entry / NDC error / clamped fov / degrees-from-vertical / |translation|. Must contain real numbers.' },
    reasoning: { type: 'string', description: 'How your independent value compares to the fixture baseline + your conclusion.' },
  },
};
const MINIMAL_SCHEMA = {
  type: 'object',
  required: ['lens', 'verdict', 'reason', 'independent_value', 'schema_fallback'],
  properties: {
    lens: { type: 'string' },
    verdict: { type: 'string', enum: ['refuted', 'refutation_failed'] },
    reason: { type: 'string', description: 'One to three sentences: conclusion + the key value you computed.' },
    independent_value: { type: ['string', 'null'], description: 'The concrete value, or null ONLY if you genuinely could not complete the analysis.' },
    schema_fallback: { type: 'boolean', description: 'Set true.' },
  },
};

const LAND_REPORT = {
  type: 'object',
  required: ['landed_file', 'fixtures_landed', 'fixtures_quarantined', 'harness_suite', 'gates', 'discovered_discrepancies', 'proposed_diff', 'committed', 'summary'],
  properties: {
    landed_file: { type: 'string' }, fixtures_landed: { type: 'number' },
    fixtures_quarantined: { type: 'array', items: { type: 'object', required: ['id', 'reason', 'failing_assertion'], properties: { id: { type: 'string' }, reason: { type: 'string' }, failing_assertion: { type: 'string' } } } },
    harness_suite: { type: 'object', required: ['command', 'result', 'passed', 'failed', 'notes'], properties: { command: { type: 'string' }, result: { type: 'string' }, passed: { type: 'number' }, failed: { type: 'number' }, notes: { type: 'string' } } },
    gates: { type: 'array', items: { type: 'object', required: ['name', 'command', 'result', 'notes'], properties: { name: { type: 'string' }, command: { type: 'string' }, result: { type: 'string' }, notes: { type: 'string' } } } },
    discovered_discrepancies: { type: 'array', items: { type: 'string' } },
    proposed_diff: { type: 'string' }, committed: { type: 'boolean' }, summary: { type: 'string' },
  },
};

const LENS_BRIEF = {
  'math-derivation': `Re-derive the baseline by a DIFFERENT method than the generator stated. Recompute the specific asserted values (matrix entries, NDC errors, clamped fovs, translation magnitudes) for the exact literal inputs. Put your numbers in independent_value. Refute if your value disagrees beyond the case epsilon; default toward refuted if you cannot reproduce the claimed baseline.`,
  'hazard-targeting': `Check the case genuinely EXERCISES the family hazard (pitch within ~1° of ±π/2; aspect genuinely non-1:1; position ≥1e5 actually losing precision; cinematics actually driving tick_cinematics). Refute (defect: mis-targeted) if not.`,
  'equivalence-bar': `Check the bar is correct: byte-equivalence must NOT be used where expected-divergence is correct; a singularity case must assert the producer's ACTUAL behaviour (finite vs NaN — verify at the literal f32 inputs). Refute (defect: wrong bar) if mismatched.`,
};

function generatePrompt(family, fixturePath) {
  return `${SHARED}\n\n# GENERATE fixture family: "${family.title}" (key: ${family.key})\n${family.brief}\n\nRead the harness file (${fixturePath}) and the sibling RenderView-level tests to match conventions and RE-VERIFY the API map against current source. Produce 3–5 concrete cases with derivable baselines (state the derivation method so a skeptic can re-derive a DIFFERENT way), correct bars, honest hazard-targeting, and complete compiling Rust #[test] code. If a hazard is not RenderView-testable, set gap_report instead of fabricating. Return ONLY the FAMILY_OUTPUT object.`;
}

function primaryPrompt(family, c, lens, attempt) {
  const retry = attempt > 1 ? `\n(RETRY ${attempt}: a prior attempt produced no recordable structured result. Do the analysis, then call StructuredOutput with ALL required fields, concrete numbers included.)\n` : '';
  return `${SHARED}\n\n# ADVERSARIAL REFUTE (lens: ${lens}) — fixture ${c.id} (family ${family.key})${retry}\nYou are a SKEPTIC. Prove this baseline WRONG on your lens; a fixture lands only if refutation FAILS. You only REPORT defects; never propose engine changes.\nLENS: ${LENS_BRIEF[lens]}\n\nFIXTURE: ${c.id}\nconfig: ${c.config}\nhazard_targeting (claim): ${c.hazard_targeting}\nequivalence_bar (claim): ${c.equivalence_bar}\nbaseline (claim): ${c.baseline}\nderivation_method (use a DIFFERENT one): ${c.derivation_method}\nrequires_gpu: ${c.requires_gpu}\ntest_code:\n\`\`\`rust\n${c.test_code}\n\`\`\`\nReturn ONLY the structured verdict.`;
}

function fallbackPrompt(family, c, lens) {
  return `${SHARED}\n\n# MINIMAL FALLBACK — fixture ${c.id} (family ${family.key}) — lens: ${lens}\nYour prior structured response could not be recorded. Return ONLY the MINIMAL record now.\nLENS: ${LENS_BRIEF[lens]}\nRules: verdict="refuted" if you can show the baseline wrong, else "refutation_failed". independent_value = the one concrete number you computed (null ONLY if you genuinely could not finish). schema_fallback=true. Do not go silent. Return ONLY the minimal record.`;
}

function landPrompt(survived, refuted, gaps, fixturePath) {
  const payload = survived.map((c, i) => `\n--- FIXTURE ${i + 1} (family ${c._family}, id ${c.id}) ---\nbar: ${c.equivalence_bar}\nrequires_gpu: ${c.requires_gpu}\ntest_code:\n\`\`\`rust\n${c.test_code}\n\`\`\``).join('\n');
  const refutedList = refuted.map((c) => `- ${c.id} (${c._family}): ${c._refute_reason}`).join('\n') || '(none)';
  const gapList = gaps.map((g) => `- family ${g.family}: ${g.gap_report}`).join('\n') || '(none)';
  return `${SHARED}\n\n# LAND surviving fixtures, RUN the suite + gates, REPORT (NO COMMIT)\n${survived.length} fixtures survived adversarial refutation. Land them into ${fixturePath} as new #[test] fns (merge needed imports). Workflow:\n1. Append the tests in a clearly-commented section, keeping existing tests intact.\n2. cargo check the test target (generous timeout); fix compile errors in TEST CODE ONLY. A fixture that would need system-under-test source changes is unlandable — quarantine + report (constraints 1,2).\n3. Run the file. Matrix fixtures need NO GPU and must pass green. GPU-gated fixtures: if no adapter, note as an ENVIRONMENT limitation, not a failure.\n4. A verified-correct fixture that FAILS at runtime: do NOT fix the engine, do NOT silently drop. Re-confirm the baseline; if still correct, this is a POTENTIAL REAL BUG — quarantine (#[ignore] with explanation), record in fixtures_quarantined + discovered_discrepancies. Escalate-don't-plow.\n5. Run the non-regression gates (cargo check --workspace + the campaign test gates) and record each result.\n6. Produce the proposed diff (git diff of the test file). DO NOT COMMIT, DO NOT git add.\nSURVIVING FIXTURES:\n${payload}\nREFUTED-AND-EXCLUDED (report; do NOT land):\n${refutedList}\nGAPS (report; follow-ups):\n${gapList}\nReturn ONLY the LAND_REPORT object (committed MUST be false).`;
}

// Hardened skeptic: 2 rich-schema + 2 minimal-fallback attempts before terminal null.
async function hardenedSkeptic(family, c, lens) {
  let attempts = 0;
  for (let i = 1; i <= 2; i++) {
    attempts++;
    const v = await agent(primaryPrompt(family, c, lens, i), { label: `refute:${c.id}:${lens}:p${i}`, phase: 'Verify', schema: SKEPTIC_SCHEMA });
    if (v) return { ...v, lens, unit_id: c.id, path: 'normal', attempts };
  }
  for (let i = 1; i <= 2; i++) {
    attempts++;
    const v = await agent(fallbackPrompt(family, c, lens), { label: `refute-fb:${c.id}:${lens}:f${i}`, phase: 'Verify', schema: MINIMAL_SCHEMA });
    if (v) return { lens, unit_id: c.id, verdict: v.verdict, confidence: 'unknown', independent_value: v.independent_value, reasoning: v.reason, schema_fallback: true, path: 'schema_fallback', attempts };
  }
  return { lens, unit_id: c.id, verdict: 'null_terminal', confidence: null, independent_value: null, reasoning: null, path: 'null_terminal', null_terminal: true, attempts };
}

async function verifyCase(family, c) {
  const lenses = ['math-derivation', 'hazard-targeting', 'equivalence-bar'];
  const verdicts = (await parallel(lenses.map((lens) => () => hardenedSkeptic(family, c, lens)))).filter(Boolean);
  const fired = verdicts.filter((v) => v.verdict === 'refuted' && (v.confidence === 'medium' || v.confidence === 'high' || v.path === 'schema_fallback'));
  const high = fired.filter((v) => v.confidence === 'high');
  const refuted = high.length >= 1 || fired.length >= 2;
  const reason = fired.map((v) => `[${v.lens}] ${v.reasoning || ''}`).join(' || ');
  return { ...c, _family: family.key, accepted: !refuted, _refute_reason: refuted ? reason : null, _verdicts: verdicts };
}

// ── RUN ──────────────────────────────────────────────────────────────────────
// `args` may override the fixture file path; defaults to the camera parity harness.
const FIXTURE_PATH = (args && args.fixturePath) || 'tools/aw_editor/tests/render_parity_harness.rs';

log(`Camera parity expansion: ${FAMILIES.length} families -> generate -> hardened adversarial-refute -> land + run (no commit). Target: ${FIXTURE_PATH}`);

const familyResults = await pipeline(
  FAMILIES,
  (family) => agent(generatePrompt(family, FIXTURE_PATH), { label: `generate:${family.key}`, phase: 'Generate', schema: FAMILY_OUTPUT }),
  async (familyOut, family) => {
    if (!familyOut) return { family: family.key, gap_report: 'generation failed (null)', cases: [] };
    const cases = Array.isArray(familyOut.cases) ? familyOut.cases : [];
    const verified = await parallel(cases.map((c) => () => verifyCase(family, c)));
    return { family: family.key, gap_report: familyOut.gap_report || null, cases: verified.filter(Boolean) };
  }
);

const families = familyResults.filter(Boolean);
const allCases = families.flatMap((f) => f.cases);
const survived = allCases.filter((c) => c.accepted);
const refuted = allCases.filter((c) => !c.accepted);
const gaps = families.filter((f) => f.gap_report).map((f) => ({ family: f.family, gap_report: f.gap_report }));

// Verifier-reliability instrumentation (the load-bearing honesty signal).
const allRecords = families.flatMap((f) => f.cases).flatMap((c) => c._verdicts || []);
const reliability = {
  logical_skeptics: allRecords.length,
  normal: allRecords.filter((r) => r.path === 'normal').length,
  schema_fallback: allRecords.filter((r) => r.path === 'schema_fallback').length,
  null_terminal: allRecords.filter((r) => r.path === 'null_terminal').length,
  raw_agent_calls: allRecords.reduce((s, r) => s + (r.attempts || 0), 0),
};

log(`Refutation complete: ${survived.length} survived, ${refuted.length} refuted-and-excluded, ${gaps.length} gap(s). Verifier: normal=${reliability.normal} fallback=${reliability.schema_fallback} null=${reliability.null_terminal}.`);

if (survived.length === 0) {
  return { phase: 'no-fixtures-survived', families: families.map((f) => ({ family: f.family, total: f.cases.length, gap_report: f.gap_report })), refuted: refuted.map((c) => ({ id: c.id, family: c._family, reason: c._refute_reason })), gaps, reliability, land_report: null };
}

phase('Land & Run');
const landReport = await agent(landPrompt(survived, refuted, gaps, FIXTURE_PATH), { label: 'land-and-run', phase: 'Land & Run', schema: LAND_REPORT });

return {
  phase: 'complete',
  families_covered: families.map((f) => ({ family: f.family, cases_generated: f.cases.length, survived: f.cases.filter((c) => c.accepted).length, refuted: f.cases.filter((c) => !c.accepted).length, gap_report: f.gap_report })),
  survived: survived.map((c) => ({ id: c.id, family: c._family, bar: c.equivalence_bar })),
  refuted: refuted.map((c) => ({ id: c.id, family: c._family, reason: c._refute_reason })),
  gaps,
  reliability,
  land_report: landReport,
};
