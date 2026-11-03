<!-- markdownlint-disable MD013 MD033 MD041 -->
<div class="astra-landing">
  <section class="astra-hero">
    <div class="astra-hero__copy">
      <span class="astra-eyebrow">MIT licensed and AI-native</span>
      <h1>Build worlds where intelligent agents are a core system, not an afterthought.</h1>
      <p class="astra-lead">
        A free, MIT-licensed Rust game engine where AI agents are a first-class runtime system.
        Deterministic simulation, tool-validated behavior, and Criterion-backed performance &mdash;
        inspectable, reproducible, and open.
      </p>
      <div class="astra-actions">
        <a class="astra-button astra-button--primary" href="./architecture/overview.md">Explore the architecture</a>
        <a class="astra-button astra-button--secondary" href="./getting-started/installation.md">Build the workspace</a>
      </div>
      <div class="astra-meta" aria-label="Adoption highlights">
        <span class="astra-meta-badge">MIT license</span>
        <span class="astra-meta-badge">Free to use, fork, and modify</span>
        <span class="astra-meta-badge">Built for engine developers</span>
      </div>
    </div>
    <div class="astra-hero__visual">
      <div class="astra-hero__frame">
        <div class="astra-hero__image-wrap">
          <img class="astra-hero__image" src="./assets/Astraweave_logo.jpg" alt="AstraWeave nebula logo">
        </div>
        <div class="astra-hero__caption">
          <strong>Scope</strong>
          <span>Deterministic ECS, AI planning and validation, replay-safe simulation.</span>
          <div class="astra-hero__highlights" aria-label="Hero highlights">
            <span>Rust 1.89.0, pinned in rust-toolchain.toml</span>
            <span>Replay hashes compared on Linux, Windows, and macOS</span>
            <span>Measured figures live on the engine status page</span>
          </div>
        </div>
      </div>
    </div>
  </section>
  <!--
    Architecture & Methodology section — added 2026-05-15 as part of the
    post-trace-campaign reconciliation. Source: ARCHITECTURE_MAP.md v0.7.0
    §0 Trace Index and §4 Workspace-Wide Structural Axioms.
  -->
  <section class="astra-section" id="methodology" aria-label="Architecture and methodology">
    <div class="astra-section-heading">
      <span class="astra-eyebrow">Architecture &amp; methodology</span>
      <h2>Documented through a forensic trace campaign, not aspirational prose.</h2>
    </div>
    <div class="astra-grid astra-grid--three">
      <article class="astra-card">
        <span class="astra-kicker">Interactive workspace map</span>
        <h3>Open the workspace map in the browser.</h3>
        <p>
          A Cytoscape.js visualization of the workspace graph, with blast-radius highlighting,
          dependency-path tracing, domain filters, focus mode, and shareable URL state.
          The crate set is the <code>members</code> list in the root <code>Cargo.toml</code>.
          On this tree that list has 140 unique paths
          (<code>astraweave-destruction</code> is entered twice).
          Copy that says 71 production crates is not this tree.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="https://lazyxeon.github.io/AstraWeave/architecture/">Open the interactive map</a>
        </div>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Architecture trace campaign</span>
        <h3>Subsystem traces, version-controlled with the code.</h3>
        <p>
          The generated table in <code>CLAUDE.md</code> lists 31 subsystem traces under
          <code>docs/architecture/</code>. The same count is recorded on the
          <a href="https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md">engine status page</a>.
          The older &ldquo;13 traces&rdquo; figure is the original campaign, not the current set.
          The consolidated synthesis is
          <a href="https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/ARCHITECTURE_MAP.md">ARCHITECTURE_MAP.md</a>.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Honest about what is in flight</span>
        <h3>Wired core and present-but-unwired surface, both visible.</h3>
        <p>
          The architecture map distinguishes the wired simulation core from crates that
          are in the workspace and not on the default sim tick. Memory, RAG, and
          coordination are in that second set. <code>AIArbiter</code> compiles only with
          <code>llm_orchestrator</code>. <code>VectorStore::search</code> is a linear scan.
          Line-count inventories on older pages were not re-counted for this site.
        </p>
      </article>
    </div>
    <div class="astra-actions" style="justify-content:center;margin-top:1.5rem;">
      <a class="astra-button astra-button--primary" href="https://lazyxeon.github.io/AstraWeave/architecture/">Open the workspace map</a>
      <a class="astra-button astra-button--secondary" href="https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/">Open the benchmark dashboard</a>
    </div>
  </section>
  <section class="astra-proof-strip" aria-label="Where the figures live">
    <article class="astra-proof-tile">
      <strong>140</strong>
      <span>unique paths in the root Cargo.toml members list (one path is repeated)</span>
    </article>
    <article class="astra-proof-tile">
      <strong>31</strong>
      <span>subsystem traces in the CLAUDE.md table</span>
    </article>
    <article class="astra-proof-tile">
      <strong>1.89.0</strong>
      <span>Rust toolchain pinned in rust-toolchain.toml</span>
    </article>
    <article class="astra-proof-tile">
      <strong>Status</strong>
      <span>Agent capacity, Miri, Kani, coverage, and frame time are dated on the engine status page. This page does not restate them.</span>
    </article>
  </section>
  <section class="astra-section" id="paths">
    <div class="astra-section-heading astra-section-heading--wide">
      <span class="astra-eyebrow">Quick navigation</span>
      <h2>Jump straight to the area you need.</h2>
    </div>
    <div class="astra-grid astra-grid--three">
      <article class="astra-card">
        <span class="astra-kicker">Getting started</span>
        <h3>Set up the workspace and run your first system.</h3>
        <ul class="astra-link-list astra-link-list--stacked">
          <li><span>Quick start</span><a href="./getting-started/quick-start.md">Open quick start</a></li>
          <li><span>Installation</span><a href="./getting-started/installation.md">Install dependencies</a></li>
          <li><span>First companion</span><a href="./getting-started/first-companion.md">Build your first AI companion</a></li>
        </ul>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Architecture</span>
        <h3>Understand the runtime model.</h3>
        <ul class="astra-link-list astra-link-list--stacked">
          <li><span>Overview</span><a href="./architecture/overview.md">Read architecture overview</a></li>
          <li><span>AI-native design</span><a href="./architecture/ai-native.md">Inspect AI-native design</a></li>
          <li><span>Validation</span><a href="./architecture/tool-validation.md">Study tool validation</a></li>
        </ul>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Core systems</span>
        <h3>Explore the engine subsystems.</h3>
        <ul class="astra-link-list astra-link-list--stacked">
          <li><span>AI system</span><a href="./core-systems/ai/index.md">Open AI systems</a></li>
          <li><span>Physics and fluids</span><a href="./core-systems/physics.md">Inspect simulation systems</a></li>
          <li><span>Terrain and navigation</span><a href="./core-systems/navigation.md">Browse world systems</a></li>
        </ul>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Examples and performance</span>
        <h3>Demos, benchmarks, and optimization.</h3>
        <ul class="astra-link-list astra-link-list--stacked">
          <li><span>Examples</span><a href="./examples/index.md">Browse examples</a></li>
          <li><span>Benchmarks</span><a href="./performance/benchmarks.md">Open benchmark dashboard</a></li>
          <li><span>Optimization</span><a href="./performance/optimization.md">Read optimization guide</a></li>
        </ul>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Engine development</span>
        <h3>Contribute and build from source.</h3>
        <ul class="astra-link-list astra-link-list--stacked">
          <li><span>Contributing</span><a href="./dev/contributing.md">Open contribution guide</a></li>
          <li><span>Building</span><a href="./dev/building.md">Build from source</a></li>
          <li><span>Testing</span><a href="./dev/testing.md">Review validation workflow</a></li>
        </ul>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">API and reference</span>
        <h3>Subsystem APIs and crate documentation.</h3>
        <ul class="astra-link-list astra-link-list--stacked">
          <li><span>API overview</span><a href="./api/index.md">Open API reference</a></li>
          <li><span>Crate map</span><a href="./reference/crates.md">Browse crate documentation</a></li>
          <li><span>CLI and config</span><a href="./reference/cli-tools.md">Inspect tools and configuration</a></li>
        </ul>
      </article>
    </div>
  </section>
  <section class="astra-section" id="proof">
    <div class="astra-section-heading">
      <span class="astra-eyebrow">Why AstraWeave</span>
      <h2>Architecture, safety, and performance you can verify.</h2>
    </div>
    <div class="astra-grid astra-grid--three">
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="10" r="4" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M8 22c0-4.4 3.6-8 8-8s8 3.6 8 8" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="6" cy="14" r="2.5" fill="none" stroke="currentColor" stroke-width="1.2" opacity=".5"/><circle cx="26" cy="14" r="2.5" fill="none" stroke="currentColor" stroke-width="1.2" opacity=".5"/><line x1="8.5" y1="14" x2="13" y2="12" stroke="currentColor" stroke-width="1" opacity=".35"/><line x1="23.5" y1="14" x2="19" y2="12" stroke="currentColor" stroke-width="1" opacity=".35"/></svg>
        <span class="astra-kicker">AI-first architecture</span>
        <h3>Perception &rarr; reasoning &rarr; planning &rarr; action, built into the runtime.</h3>
        <p>
          World snapshots, plan intents, tool validation, behavior trees, GOAP, and LLM-backed
          planning all plug into a deterministic loop.
        </p>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><rect x="7" y="7" width="18" height="18" rx="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M13 16l2.5 2.5L19.5 13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><line x1="16" y1="3" x2="16" y2="7" stroke="currentColor" stroke-width="1.2" opacity=".5"/><line x1="16" y1="25" x2="16" y2="29" stroke="currentColor" stroke-width="1.2" opacity=".5"/><line x1="3" y1="16" x2="7" y2="16" stroke="currentColor" stroke-width="1.2" opacity=".5"/><line x1="25" y1="16" x2="29" y2="16" stroke="currentColor" stroke-width="1.2" opacity=".5"/></svg>
        <span class="astra-kicker">Deterministic and safe</span>
        <h3>Replay hashes, validator-gated actions, and proof harnesses.</h3>
        <p>
          Required replay cases are compared on Linux, Windows, and macOS.
          Miri and Kani counts are dated measurements on the engine status page.
          This page does not repeat those counts.
        </p>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><rect x="4" y="18" width="5" height="10" rx="1" fill="currentColor" opacity=".25"/><rect x="11" y="12" width="5" height="16" rx="1" fill="currentColor" opacity=".4"/><rect x="18" y="8" width="5" height="20" rx="1" fill="currentColor" opacity=".55"/><rect x="25" y="4" width="5" height="24" rx="1" fill="currentColor" opacity=".7"/><line x1="4" y1="29" x2="30" y2="29" stroke="currentColor" stroke-width="1" opacity=".3"/></svg>
        <span class="astra-kicker">Benchmarked subsystems</span>
        <h3>Performance figures stay on the engine status page.</h3>
        <p>
          Frame time, agent capacity, and older Criterion snapshots are recorded with
          their commit and date there. This site does not promote an undated bar chart
          as the latest run.
        </p>
      </article>
    </div>
    <div class="astra-band">
      <div class="astra-section-heading astra-section-heading--compact">
        <span class="astra-eyebrow">Engine loop</span>
        <h2>Perception &rarr; reasoning &rarr; planning &rarr; validation &rarr; action.</h2>
      </div>
      <div class="astra-flow">
        <div class="astra-node">
          <strong>Perception</strong>
          <span>World snapshots from deterministic ECS state.</span>
        </div>
        <div class="astra-node">
          <strong>Reasoning</strong>
          <span>Behavior trees, utility systems, GOAP, or LLM logic.</span>
        </div>
        <div class="astra-node">
          <strong>Planning</strong>
          <span>Action sequences with costs, priorities, and fallbacks.</span>
        </div>
        <div class="astra-node">
          <strong>Validation</strong>
          <span>Cooldowns, LOS, pathing, and sandbox constraints.</span>
        </div>
        <div class="astra-node">
          <strong>Action</strong>
          <span>Approved commands flow into simulation, physics, and rendering.</span>
        </div>
      </div>
    </div>
  </section>
  <section class="astra-section" id="systems">
    <div class="astra-section-heading astra-section-heading--wide">
      <span class="astra-eyebrow">What ships today</span>
      <h2>A focused stack for intelligent, simulation-heavy games.</h2>
    </div>
    <div class="astra-grid astra-grid--three">
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="16" r="4" fill="currentColor" opacity=".4"/><circle cx="16" cy="16" r="10" fill="none" stroke="currentColor" stroke-width="1.3" stroke-dasharray="3,3"/><circle cx="6" cy="10" r="2" fill="currentColor" opacity=".25"/><circle cx="26" cy="10" r="2" fill="currentColor" opacity=".25"/><circle cx="6" cy="22" r="2" fill="currentColor" opacity=".25"/><circle cx="26" cy="22" r="2" fill="currentColor" opacity=".25"/><line x1="8" y1="10" x2="12" y2="14" stroke="currentColor" stroke-width="1" opacity=".3"/><line x1="24" y1="10" x2="20" y2="14" stroke="currentColor" stroke-width="1" opacity=".3"/><line x1="8" y1="22" x2="12" y2="18" stroke="currentColor" stroke-width="1" opacity=".3"/><line x1="24" y1="22" x2="20" y2="18" stroke="currentColor" stroke-width="1" opacity=".3"/></svg>
        <span class="astra-kicker">AI orchestration</span>
        <h3>Seven hello_companion modes, feature-gated.</h3>
        <p>
          A default <code>cargo run -p hello_companion</code> uses <code>RuleOrchestrator</code>.
          Behavior Tree, Utility, and Ensemble need <code>llm</code>. LLM and Hybrid need
          <code>ollama</code>. The arbiter flag needs <code>llm_orchestrator</code> and does not
          place <code>AIArbiter</code> on the deterministic tick.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="./architecture/ai-native.md">AI-native design</a>
          <a class="astra-chip" href="./api/ai.md">AI API</a>
        </div>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><rect x="4" y="4" width="10" height="10" rx="2" fill="currentColor" opacity=".3"/><rect x="18" y="4" width="10" height="10" rx="2" fill="currentColor" opacity=".2"/><rect x="4" y="18" width="10" height="10" rx="2" fill="currentColor" opacity=".2"/><rect x="18" y="18" width="10" height="10" rx="2" fill="currentColor" opacity=".4"/><line x1="14" y1="9" x2="18" y2="9" stroke="currentColor" stroke-width="1" opacity=".4"/><line x1="9" y1="14" x2="9" y2="18" stroke="currentColor" stroke-width="1" opacity=".4"/><line x1="23" y1="14" x2="23" y2="18" stroke="currentColor" stroke-width="1" opacity=".4"/></svg>
        <span class="astra-kicker">Deterministic ECS</span>
        <h3>Ordered simulation for replay, tooling, and scale.</h3>
        <p>
          Archetype storage, system staging, iteration guarantees, and event channels
          form a reproducible simulation backbone.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="./architecture/ecs.md">ECS architecture</a>
          <a class="astra-chip" href="./api/ecs.md">ECS API</a>
        </div>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><rect x="6" y="8" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><line x1="6" y1="24" x2="26" y2="24" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="12" cy="15" r="2" fill="currentColor" opacity=".3"/><circle cx="20" cy="15" r="2" fill="currentColor" opacity=".3"/><line x1="14" y1="15" x2="18" y2="15" stroke="currentColor" stroke-width="1" opacity=".3"/></svg>
        <span class="astra-kicker">Rendering</span>
        <h3>wgpu-based rendering with real engine workloads.</h3>
        <p>
          PBR materials, clustered lighting, GPU skinning, post-processing, and LOD tooling.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="./core-systems/rendering.md">Rendering systems</a>
          <a class="astra-chip" href="./api/render.md">Render API</a>
        </div>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="16" r="11" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M16 5v6M16 21v6" stroke="currentColor" stroke-width="1.2" opacity=".4"/><path d="M5 16h6M21 16h6" stroke="currentColor" stroke-width="1.2" opacity=".4"/><path d="M10 22l3-3M19 13l3-3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
        <!--
          Source: ARCHITECTURE_MAP.md §5.1 Dormant-Code Inventory. Fluids is
          dormant in-design (84.5K LoC, only examples/fluids_demo consumes it,
          no production game-loop crate depends on it). SpatialHash module
          is dormant per physics.md §6 — actual broadphase is Rapier's
          DefaultBroadPhase. Card revised to describe what is actually wired.
        -->
        <span class="astra-kicker">Physics and movement</span>
        <h3>Rapier-backed dynamics with character controllers and spatial queries.</h3>
        <p>
          Rigid bodies, character controllers, and Rapier collision. <code>PhysicsWorld</code>
          uses Rapier's <code>DefaultBroadPhase</code>. The in-crate <code>SpatialHash</code>
          is not that broadphase. <code>astraweave-fluids</code> is not the game-loop solver.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="./core-systems/physics.md">Physics</a>
          <a class="astra-chip" href="./core-systems/fluids.md">Fluids (research)</a>
        </div>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><polygon points="16,4 28,12 28,24 16,28 4,24 4,12" fill="none" stroke="currentColor" stroke-width="1.5"/><line x1="16" y1="4" x2="16" y2="28" stroke="currentColor" stroke-width="1" opacity=".25"/><line x1="4" y1="12" x2="28" y2="12" stroke="currentColor" stroke-width="1" opacity=".25"/><circle cx="16" cy="16" r="2" fill="currentColor" opacity=".4"/></svg>
        <span class="astra-kicker">Navigation and world systems</span>
        <h3>Navmesh, terrain, scene streaming, and gameplay layers.</h3>
        <p>
          Terrain generation, navigation meshes, crafting, quests, dialogue, and procedural content.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="./core-systems/navigation.md">Navigation</a>
          <a class="astra-chip" href="./core-systems/terrain.md">Terrain</a>
        </div>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><path d="M10 8h12a2 2 0 012 2v12a2 2 0 01-2 2H10a2 2 0 01-2-2V10a2 2 0 012-2z" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M14 14l2 2 4-4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="12" x2="8" y2="12" stroke="currentColor" stroke-width="1.2" opacity=".4"/><line x1="4" y1="16" x2="8" y2="16" stroke="currentColor" stroke-width="1.2" opacity=".4"/><line x1="4" y1="20" x2="8" y2="20" stroke="currentColor" stroke-width="1.2" opacity=".4"/></svg>
        <span class="astra-kicker">Tooling and integration</span>
        <h3>Examples, the editor, and a C SDK crate.</h3>
        <p>
          Workspace examples, <code>cargo editor</code> for <code>aw_editor</code>, and
          <code>astraweave-sdk</code>. The editor aliases are <code>editor</code>,
          <code>editor-release</code>, and <code>editor-dev</code>.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="./examples/index.md">Examples</a>
          <a class="astra-chip" href="./api/index.md">API docs</a>
        </div>
      </article>
    </div>
  </section>
  <section class="astra-section">
    <div class="astra-split">
      <article class="astra-card astra-card--panel">
        <span class="astra-kicker">Where numbers live</span>
        <h3>Measurements are on the engine status page.</h3>
        <p>
          Agent capacity, validation throughput, CPU frame samples, Miri, Kani, and coverage
          are dated on the
          <a href="https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md">engine status page</a>.
          A February 2026 bar chart that this page used to call the latest run is removed.
          The historical write-up is
          <a href="https://github.com/lazyxeon/AstraWeave/blob/main/docs/masters/MASTER_BENCHMARK_REPORT.md">MASTER_BENCHMARK_REPORT.md</a>.
          It is not a fresh capture.
        </p>
        <p>
          <code>#[kani::proof]</code> appears 69 times in this tree. Engine status records
          that same count as of 2026-06-13 and says it was not a fresh proof run.
          The older &ldquo;71+&rdquo; harness figure is not that count.
        </p>
      </article>
    </div>
    <div class="astra-split astra-split--offset">
      <article class="astra-card astra-card--panel">
        <span class="astra-kicker">Quality posture</span>
        <h3>Verification and testing across the stack.</h3>
        <ul class="astra-list">
          <li><span>Coverage, Miri, and Kani</span><strong>Dated on the engine status page</strong></li>
          <li><span>Kani attributes in this tree</span><strong>69 <code>#[kani::proof]</code> matches</strong></li>
          <li><span>Replay compare</span><strong>Linux, Windows, and macOS</strong></li>
          <li><span>Desktop CI</span><strong>Windows, Linux, macOS</strong></li>
        </ul>
      </article>
      <article class="astra-card astra-card--panel">
        <span class="astra-kicker">Developer routes</span>
        <h3>Pick an entry point.</h3>
        <ul class="astra-link-list">
          <li><span>System design</span><a href="./architecture/overview.md">Open architecture overview</a></li>
          <li><span>Performance data</span><a href="./performance/benchmarks.md">Open benchmarks</a></li>
          <li><span>Build and run</span><a href="./getting-started/quick-start.md">Read quick start</a></li>
          <li><span>Examples and demos</span><a href="./examples/index.md">Browse examples</a></li>
          <li><span>Contributing workflow</span><a href="./dev/contributing.md">Open contribution guide</a></li>
          <li><span>Reference implementation</span><a href="./veilweaver/overview.md">Inspect Veilweaver</a></li>
        </ul>
      </article>
    </div>
  </section>
  <section class="astra-section" id="audiences">
    <div class="astra-section-heading astra-section-heading--wide">
      <span class="astra-eyebrow">Use cases</span>
      <h2>Where AstraWeave fits best.</h2>
    </div>
    <div class="astra-grid astra-grid--four">
      <article class="astra-card">
        <span class="astra-kicker">RPGs and immersive sims</span>
        <h3>Companions, directors, and systemic encounters.</h3>
        <p>
          Projects where NPCs need to observe, plan, and react with more depth than
          state-machine scripting allows.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Server-authoritative multiplayer</span>
        <h3>Validation and replayability matter.</h3>
        <p>
          Deterministic simulation and tool-gated actions for anti-cheat,
          reproducibility, and replay validation.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Research and prototyping</span>
        <h3>Benchmarkable AI-native architecture.</h3>
        <p>
          Test agent scale, planning strategies, and hybrid AI control
          under measurable conditions.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Embedded engine teams</span>
        <h3>Rust core with a C ABI.</h3>
        <p>
          Adopt focused subsystems through the modular crate structure
          instead of committing to the whole stack.
        </p>
      </article>
    </div>
  </section>
  <section class="astra-section" id="lineages">
    <div class="astra-section-heading astra-section-heading--wide">
      <span class="astra-eyebrow">Design lineages</span>
      <h2>Games this engine could help realize.</h2>
    </div>
    <div class="astra-grid astra-grid--three">
      <article class="astra-card">
        <span class="astra-kicker">Colony and world simulation</span>
        <h3>In the lineage of Dwarf Fortress or RimWorld.</h3>
        <p>
          Agent autonomy, world-state memory, logistics, and emergent story generation.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">4X and grand strategy</span>
        <h3>In the lineage of Civilization or Crusader Kings.</h3>
        <p>
          Multi-agent diplomacy, advisor systems, strategic planners, and explainable AI.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Tactical command games</span>
        <h3>In the lineage of X-COM or Battle Brothers.</h3>
        <p>
          Tool validation, cover awareness, action planning, and replay-safe combat loops.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Systemic sandboxes</span>
        <h3>In the lineage of Kenshi or Mount and Blade.</h3>
        <p>
          Large numbers of autonomous actors with persistent world consequences.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Immersive sims and party RPGs</span>
        <h3>In the lineage of Deus Ex or Dragon Age.</h3>
        <p>
          Companion decisions, quest reactivity, systemic encounters, and director-style orchestration.
        </p>
      </article>
      <article class="astra-card">
        <span class="astra-kicker">Rights-holder remakes</span>
        <h3>Licensed ports or original successors.</h3>
        <p>
          Rebuild ambitious systemic designs with modern AI-native architecture.
        </p>
      </article>
    </div>
  </section>
  <section class="astra-cta">
    <div>
      <span class="astra-eyebrow">Next step</span>
      <h2>Clone, build, and decide from evidence.</h2>
      <p class="astra-note astra-note--light">
        AstraWeave is free and MIT licensed. Evaluate, adopt subsystems, or contribute back
        without platform lock-in.
      </p>
    </div>
    <div class="astra-actions astra-actions--stacked">
      <a class="astra-button astra-button--primary" href="./getting-started/installation.md">Install dependencies</a>
      <a class="astra-button astra-button--secondary" href="./performance/benchmarks.md">Read benchmark report</a>
      <a class="astra-button astra-button--secondary" href="./dev/contributing.md">Contribute to the engine</a>
    </div>
  </section>
</div>
<!-- markdownlint-enable MD013 MD033 MD041 -->
