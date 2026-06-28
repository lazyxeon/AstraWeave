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
            <span>12,700+ agents at 60 FPS</span>
            <span>1,059 Miri tests, 0 undefined behavior</span>
            <span>71+ Kani proof harnesses</span>
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
        <h3>Explore 71 production crates and 188 dependencies in your browser.</h3>
        <p>
          A Cytoscape.js visualization with blast-radius highlighting, dependency-path tracing,
          domain filters, focus mode, an 8-step guided tour, and shareable URL state.
          Click any node for crate detail; click any edge for the load-bearing types
          crossing the boundary.
        </p>
        <div class="astra-chip-row">
          <a class="astra-chip" href="https://lazyxeon.github.io/AstraWeave/architecture/">Open the interactive map</a>
        </div>
      </article>
      <article class="astra-card">
        <!-- Source: ARCHITECTURE_MAP.md §0 (13 traces enumerated) -->
        <span class="astra-kicker">Architecture trace campaign</span>
        <h3>13 evidence-grounded subsystem traces, version-controlled with the code.</h3>
        <p>
          Each trace is a forensic reference: §5 file map, §6 conflict map, §7 decision log,
          §8 invariants, §11 open questions. Traces document how each subsystem
          <em>actually</em> works, separate from how older documentation aspirationally
          describes it. The consolidated synthesis is the
          <a href="https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/ARCHITECTURE_MAP.md">2,500&#8209;line architecture map</a>.
        </p>
      </article>
      <article class="astra-card">
        <!-- Source: ARCHITECTURE_MAP.md §5 (Dormant-Code Inventory ~200K LoC) -->
        <span class="astra-kicker">Honest about what is in flight</span>
        <h3>Load-bearing core vs. designed-but-not-wired surface, both visible.</h3>
        <p>
          The map and visualization explicitly distinguish the production-wired core
          (ECS, AI pipeline, rendering, terrain, editor) from
          ~200K&nbsp;LoC of dormant-but-designed surface (research-grade fluids,
          LLM hardening, advanced GOAP, memory pipeline). Both are documented;
          neither is sold as something it is not.
        </p>
      </article>
    </div>
    <div class="astra-actions" style="justify-content:center;margin-top:1.5rem;">
      <a class="astra-button astra-button--primary" href="https://lazyxeon.github.io/AstraWeave/architecture/">Open the workspace map</a>
      <a class="astra-button astra-button--secondary" href="https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/">Open the benchmark dashboard</a>
    </div>
  </section>
  <section class="astra-proof-strip" aria-label="Key proof points">
    <article class="astra-proof-tile">
      <strong>12,700+</strong>
      <span>validated agents at 60 FPS on modest consumer hardware</span>
    </article>
    <!--
      Source: cargo metadata --no-deps (2026-05-13) reports 129 workspace
      members; ARCHITECTURE_MAP.md §1 cites 143 as a higher-level count;
      reconciliation note in workspace_map.html surfaces the difference.
      Test count consolidates editor (~9,397 annotations per aw_editor.md §10)
      with workspace test totals.
    -->
    <article class="astra-proof-tile">
      <strong>129</strong>
      <span>workspace members with comprehensive test coverage</span>
    </article>
    <article class="astra-proof-tile">
      <strong>1,059</strong>
      <span>Miri tests with zero undefined behavior</span>
    </article>
    <article class="astra-proof-tile">
      <strong>0.71 ms</strong>
      <span>total frame time at 1,000 entities (profiling_demo; mimalloc default — 0.97 ms on system alloc). 2.70 ms was the Week-8 target.</span>
    </article>
    <article class="astra-proof-tile">
      <strong>71+</strong>
      <span>Kani proof harnesses across safety-critical crates</span>
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
        <h3>Bit-identical replay, validator-gated actions, and formal verification.</h3>
        <p>
          Core unsafe paths are exercised under Miri. Kani proofs back critical ECS, math,
          and SDK invariants. Replay validation is built in, not bolted on.
        </p>
      </article>
      <article class="astra-card">
        <svg class="astra-card-icon" viewBox="0 0 32 32" aria-hidden="true"><rect x="4" y="18" width="5" height="10" rx="1" fill="currentColor" opacity=".25"/><rect x="11" y="12" width="5" height="16" rx="1" fill="currentColor" opacity=".4"/><rect x="18" y="8" width="5" height="20" rx="1" fill="currentColor" opacity=".55"/><rect x="25" y="4" width="5" height="24" rx="1" fill="currentColor" opacity=".7"/><line x1="4" y1="29" x2="30" y2="29" stroke="currentColor" stroke-width="1" opacity=".3"/></svg>
        <span class="astra-kicker">Benchmarked subsystems</span>
        <h3>Every performance claim links to a reproducible Criterion measurement.</h3>
        <p>
          ECS, AI planning, physics, rendering frame times, and SIMD throughput are all
          measured with specific numbers, not broad adjectives.
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
        <h3>Six validated modes plus hybrid arbiters.</h3>
        <p>
          Classical planners, behavior trees, utility logic, LLM orchestration,
          ensemble patterns, and hybrid arbiters.
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
          Rigid bodies, character controllers, collision detection, raycasting and shape
          casting, joints, CCD. Fluid simulation (PBD SPH) and a SpatialHash broadphase
          exist as <em>research surface</em> in the workspace but are not currently
          production-wired — see the architecture map for the dormant-surface inventory.
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
        <h3>Example suite, editor tooling, and a C ABI for embedding.</h3>
        <p>
          A large example suite, editor workflows, and a stable C SDK layer.
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
        <span class="astra-kicker">Evidence-backed metrics</span>
        <h3>Current measurements from the codebase.</h3>
        <ul class="astra-list">
          <li><span>Agent capacity at 60 FPS</span><strong>12,700+</strong></li>
          <li><span>AI validation throughput</span><strong>6.48M checks/sec</strong></li>
          <li><span>Frame time at 1,000 entities</span><strong>0.71 ms (mimalloc) / 0.97 ms (system)</strong></li>
          <li><span>ECS world creation</span><strong>25.8 ns</strong></li>
          <li><span>Character move cost</span><strong>114 ns</strong></li>
          <li><span>SIMD batch over 10k entities</span><strong>9.879 us</strong></li>
        </ul>
      </article>
      <article class="astra-bench-card" aria-label="Top benchmark results">
        <div class="astra-bench-header">
          <div>
            <div class="astra-bench-title">Top Benchmarks &middot; Latest Run</div>
            <div class="astra-bench-subtitle">Criterion.rs statistical benchmarking &middot; Feb 2026</div>
          </div>
          <span class="astra-bench-badge"><span class="astra-bench-badge-dot"></span>Stable baseline</span>
        </div>
        <div class="astra-bench-legend">
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#4ea8de"></span>Bake 10K Triangles</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#e8915a"></span>E2E Plan Gen &middot; Cache Miss</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#fbbf24"></span>Cache Latency &middot; 200ms</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#a78bfa"></span>Light Binning &middot; High</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#22d3ee"></span>Bloom Upsample &middot; Mip 0</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#f472b6"></span>Light Binning &middot; 5K</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#f87171"></span>Cache Latency &middot; 100ms</span>
          <span class="astra-bench-li"><span class="astra-bench-sw" style="background:#5bc9a0"></span>Game Loop &middot; 5K Stress</span>
        </div>
        <svg class="astra-bench-svg" viewBox="0 0 900 390" xmlns="http://www.w3.org/2000/svg" aria-label="Horizontal bar chart of benchmark results">
          <rect width="900" height="390" fill="#161922"/>
          <line x1="310" y1="14" x2="310" y2="350" stroke="rgba(255,255,255,0.04)"/>
          <line x1="410" y1="14" x2="410" y2="350" stroke="rgba(255,255,255,0.04)"/>
          <line x1="510" y1="14" x2="510" y2="350" stroke="rgba(255,255,255,0.04)"/>
          <line x1="610" y1="14" x2="610" y2="350" stroke="rgba(255,255,255,0.04)"/>
          <line x1="710" y1="14" x2="710" y2="350" stroke="rgba(255,255,255,0.04)"/>
          <text x="310" y="372" fill="#555a6e" font-size="10" text-anchor="middle" style="font-family:Consolas,monospace">100 ms</text>
          <text x="410" y="372" fill="#555a6e" font-size="10" text-anchor="middle" style="font-family:Consolas,monospace">200 ms</text>
          <text x="510" y="372" fill="#555a6e" font-size="10" text-anchor="middle" style="font-family:Consolas,monospace">300 ms</text>
          <text x="610" y="372" fill="#555a6e" font-size="10" text-anchor="middle" style="font-family:Consolas,monospace">400 ms</text>
          <text x="710" y="372" fill="#555a6e" font-size="10" text-anchor="middle" style="font-family:Consolas,monospace">500 ms</text>
          <defs>
            <linearGradient id="blueBarGlow" x1="0" y1="0" x2="1" y2="0"><stop offset="0%" stop-color="#4ea8de"/><stop offset="100%" stop-color="#4ea8de" stop-opacity="0.6"/></linearGradient>
          </defs>
          <text x="200" y="39" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Bake 10K Triangles</text>
          <rect x="210" y="22" width="548" height="28" rx="4" fill="url(#blueBarGlow)" opacity="0.9"/>
          <text x="766" y="40" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">548.16 ms</text>
          <text x="200" y="81" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">E2E Plan Gen · Cache Miss</text>
          <rect x="210" y="64" width="219" height="28" rx="4" fill="#e8915a" opacity="0.85"/>
          <text x="435" y="82" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">218.79 ms</text>
          <text x="200" y="123" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Cache Latency · 200ms</text>
          <rect x="210" y="106" width="209" height="28" rx="4" fill="#fbbf24" opacity="0.85"/>
          <text x="425" y="124" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">209.26 ms</text>
          <text x="200" y="165" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Light Binning · High</text>
          <rect x="210" y="148" width="176" height="28" rx="4" fill="#a78bfa" opacity="0.85"/>
          <text x="392" y="166" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">176.06 ms</text>
          <text x="200" y="207" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Bloom Upsample · Mip 0</text>
          <rect x="210" y="190" width="173" height="28" rx="4" fill="#22d3ee" opacity="0.85"/>
          <text x="389" y="208" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">172.55 ms</text>
          <text x="200" y="249" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Light Binning · 5K</text>
          <rect x="210" y="232" width="113" height="28" rx="4" fill="#f472b6" opacity="0.85"/>
          <text x="329" y="250" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">113.34 ms</text>
          <text x="200" y="291" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Cache Latency · 100ms</text>
          <rect x="210" y="274" width="109" height="28" rx="4" fill="#f87171" opacity="0.85"/>
          <text x="325" y="292" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">108.52 ms</text>
          <text x="200" y="333" fill="#8b90a0" font-size="11" text-anchor="end" style="font-family:'Segoe UI',system-ui,sans-serif">Game Loop · 5K Stress</text>
          <rect x="210" y="316" width="94" height="28" rx="4" fill="#5bc9a0" opacity="0.85"/>
          <text x="310" y="334" fill="#e2e4ea" font-size="10" style="font-family:Consolas,monospace">93.91 ms</text>
        </svg>
        <div class="astra-bench-footer">
          <span class="astra-bench-caption">
            All measurements are p50 medians from Criterion.rs with &ge;100 iterations per benchmark and 95% confidence intervals.
          </span>
          <span class="astra-bench-source">criterion &middot; cargo bench</span>
        </div>
      </article>
    </div>
    <div class="astra-split astra-split--offset">
      <article class="astra-card astra-card--panel">
        <span class="astra-kicker">Quality posture</span>
        <h3>Verification and testing across the stack.</h3>
        <ul class="astra-list">
          <li><span>Weighted line coverage</span><strong>59.3% across measured crates</strong></li>
          <li><span>High-coverage crates</span><strong>14 crates at 85%+</strong></li>
          <li><span>Miri validation</span><strong>1,059 tests, 0 UB</strong></li>
          <li><span>Kani verification</span><strong>71+ harnesses</strong></li>
          <li><span>Prompt mutation testing</span><strong>100% adjusted kill rate</strong></li>
          <li><span>Desktop targets</span><strong>Windows, Linux, macOS</strong></li>
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
