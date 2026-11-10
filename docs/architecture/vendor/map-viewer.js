"use strict";
// ============================================================
// Load workspace data
// ============================================================
const workspaceData = JSON.parse(document.getElementById('workspace-data').textContent);
const NODES = workspaceData.nodes;
// Integrity guard. cytoscape() throws on an edge whose endpoint is not a node; the throw
// aborts this script and the map renders empty (live 2026-10-15 to 2026-11-07). Drop such
// edges so the rest of the map renders, and report them in the header and the console.
// `aw_trace_sync --check` is the real gate; this keeps one bad record from blanking the map.
const NODE_IDS = new Set(NODES.map(n => n.id));
const DANGLING_EDGES = workspaceData.edges.filter(
  e => !NODE_IDS.has(e.source) || !NODE_IDS.has(e.target));
const EDGES = workspaceData.edges.filter(
  e => NODE_IDS.has(e.source) && NODE_IDS.has(e.target));
if (DANGLING_EDGES.length > 0) {
  console.error('workspace-data integrity: dropped ' + DANGLING_EDGES.length +
    ' edge(s) whose endpoint is not a node:',
    DANGLING_EDGES.map(e => e.source + ' -> ' + e.target));
}
const DOMAINS = workspaceData.domains;
const STORIES = workspaceData.stories;
const RECONCILE = workspaceData.reconciliationNotes;
const META = workspaceData.metadata;

const domainColor = Object.fromEntries(DOMAINS.map(d => [d.id, d.color]));

// ============================================================
// Trace-link base URL
//   Local (file://)   → relative `./` links (the .md sits next to the .html)
//   GitHub Pages      → GitHub blob view (browsers render .md there;
//                        Pages serves .md as text/plain which is ugly)
//   Any other host    → relative (preserves authored layout)
// ============================================================
const REPO_BLOB = 'https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/';
const TRACE_BASE = (function() {
  try {
    const params = new URLSearchParams(window.location.search || '');
    if (params.get('mode') === 'pages') return REPO_BLOB;
    if (params.get('mode') === 'local') return './';
    const host = window.location.hostname || '';
    if (host.endsWith('.github.io')) return REPO_BLOB;
  } catch (_) {}
  return './';
})();
// Rewrite static .md links in the page (header/footer) once on load. Runs
// whenever TRACE_BASE points at a GitHub blob URL — i.e. when served from
// github.io, or when `?mode=pages` is in the query string for preview.
function rewriteStaticTraceLinks() {
  if (TRACE_BASE === './') return;
  document.querySelectorAll('a[href*=".md"], a[href$="/_meta/"]').forEach(a => {
    const href = a.getAttribute('href') || '';
    let m = href.match(/^\.\/(_meta\/[^\/]+\.md)(#.*)?$/);
    if (m) { a.setAttribute('href', REPO_BLOB + m[1] + (m[2] || '')); return; }
    m = href.match(/^\.\/_meta\/?$/);
    if (m) { a.setAttribute('href', REPO_BLOB + '_meta/'); return; }
    m = href.match(/^\.\/?([^\/]+\.md)(#.*)?$/);
    if (m) { a.setAttribute('href', REPO_BLOB + m[1] + (m[2] || '')); return; }
  });
}
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', rewriteStaticTraceLinks);
} else {
  rewriteStaticTraceLinks();
}

// Pre-compute neighbor maps for fast in/out edge lookup
const incoming = new Map();
const outgoing = new Map();
NODES.forEach(n => { incoming.set(n.id, []); outgoing.set(n.id, []); });
EDGES.forEach(e => {
  if (outgoing.has(e.source)) outgoing.get(e.source).push(e);
  if (incoming.has(e.target)) incoming.get(e.target).push(e);
});

// ============================================================
// Header counts + reconciliation list
// ============================================================
document.getElementById('node-count').textContent = NODES.length;
document.getElementById('edge-count').textContent = EDGES.length;
if (DANGLING_EDGES.length > 0) {
  const pill = document.getElementById('integrity-pill');
  pill.textContent = '⚠ ' + DANGLING_EDGES.length + ' edge(s) dropped: endpoint not on map';
  pill.title = DANGLING_EDGES.map(e => e.source + ' → ' + e.target).join('\n');
  pill.style.display = '';
}
const reconList = document.getElementById('reconciliation-notes');
RECONCILE.forEach(text => {
  const li = document.createElement('li');
  li.textContent = text;
  reconList.appendChild(li);
});

// ============================================================
// How-to-use toggle
// ============================================================
document.getElementById('htu-toggle').addEventListener('click', (e) => {
  const body = document.getElementById('htu-body');
  const open = body.classList.toggle('open');
  e.target.setAttribute('aria-expanded', open ? 'true' : 'false');
  e.target.textContent = open ? 'How to use ↑' : 'How to use ↓';
});

// ============================================================
// Domain filter chips
// ============================================================
// v2.1 filter fix: v2 mechanism was correct (chips toggled, nodes hidden), but
// the model was inverted from typical filter UX — chips defaulted to ON, click
// REMOVED a domain. User reported "the graph doesn't change," which was
// actually "unchecking one chip yielded 'hide that domain' rather than the
// expected 'show only this domain.'" v2.1 inverts the model:
//   * empty enabledDomains set  →  no filter, show all
//   * partial set               →  include-only, hide non-matching
//   * full set                  →  no filter, show all (equivalent to empty)
// Chips default to "off" (dim). Click toggles them "on" (highlighted accent).
const domainFilters = document.getElementById('domain-filters');
const enabledDomains = new Set();   // v2.1: empty by default = no filter active
const domLabelSpan = document.createElement('span');
domLabelSpan.className = 'tb-label';
domLabelSpan.textContent = 'Domains';
domainFilters.appendChild(domLabelSpan);
DOMAINS.forEach(d => {
  const chip = document.createElement('button');
  chip.className = 'domain-chip';
  chip.setAttribute('aria-pressed', 'false');
  chip.setAttribute('data-domain', d.id);
  chip.innerHTML = `<span class="dot" style="background:${d.color}"></span>${d.label}`;
  chip.addEventListener('click', () => {
    if (chip.disabled) return;
    const nowOn = chip.classList.toggle('on');
    chip.setAttribute('aria-pressed', nowOn ? 'true' : 'false');
    if (nowOn) enabledDomains.add(d.id);
    else enabledDomains.delete(d.id);
    applyFilters();
  });
  domainFilters.appendChild(chip);
});
// Small hint that appears only when filter is active
const filterHint = document.createElement('span');
filterHint.className = 'filter-hint';
filterHint.id = 'filter-hint';
filterHint.textContent = '';
domainFilters.appendChild(filterHint);

function isDomainFilterActive() {
  // Active iff non-empty AND proper subset of all domains.
  return enabledDomains.size > 0 && enabledDomains.size < DOMAINS.length;
}
function updateFilterHint() {
  const hint = document.getElementById('filter-hint');
  if (!hint) return;
  if (isDomainFilterActive()) {
    hint.textContent = `(showing ${enabledDomains.size} of ${DOMAINS.length})`;
  } else {
    hint.textContent = '';
  }
}

// ============================================================
// Story dropdown
// ============================================================
const storySelect = document.getElementById('story-select');
STORIES.forEach(s => {
  const opt = document.createElement('option');
  opt.value = s.id;
  opt.textContent = s.title;
  storySelect.appendChild(opt);
});

// ============================================================
// Build Cytoscape elements
// ============================================================
function sizeForBand(band) {
  switch (band) {
    case 'extra-large': return 92;
    case 'large':       return 64;
    case 'medium':      return 44;
    default:            return 28;
  }
}
function fontForBand(band) {
  switch (band) {
    case 'extra-large': return 13;
    case 'large':       return 12;
    case 'medium':      return 10;
    default:            return 9;
  }
}

const cyElements = [];

// Domain compound parents
DOMAINS.forEach(d => {
  cyElements.push({
    data: { id: `dom-${d.id}`, label: d.label, isCompound: true, domain: d.id },
    classes: 'compound',
  });
});

// Nodes
NODES.forEach(n => {
  const overlays = [];
  if (n.silentFailures.length > 0) overlays.push('⚠');
  if (n.trace) overlays.push('📄');
  if (n.aspirationalDocs) overlays.push('📜');

  cyElements.push({
    data: {
      id: n.id,
      label: n.label,
      parent: `dom-${n.domain}`,
      domain: n.domain,
      domainLabel: n.domainLabel,
      locBand: n.locBand,
      locApprox: n.locApprox,
      status: n.status,
      statusCategory: n.statusCategory,
      statusEvidence: n.statusEvidence,
      trace: n.trace,
      silentFailures: n.silentFailures,
      aspirationalDocs: n.aspirationalDocs,
      openQuestions: n.openQuestions,
      overlays: overlays.join(' '),
      hasSilent: n.silentFailures.length > 0,
      hasTrace: !!n.trace,
      hasAspirational: !!n.aspirationalDocs,
      isExample: n.isExample,
      exampleList: n.exampleList || null,
      size: sizeForBand(n.locBand),
      font: fontForBand(n.locBand),
      _color: domainColor[n.domain] || '#888',
    },
    classes: `node status-${n.status} band-${n.locBand}`,
  });
});

// v2.1: pre-compute the per-edge gradient stops + arrow color and store on
// edge data. Avoids per-render function calls in the Cytoscape style block,
// which regressed initial-render time by ~2.5s in headless tests.
const NODE_DOMAIN_BY_ID = new Map(NODES.map(n => [n.id, n.domain]));
function buildEdgeGradient(srcDom, tgtDom) {
  const sc = domainColor[srcDom] || '#6e7681';
  const tc = domainColor[tgtDom] || '#6e7681';
  return sc + ' ' + tc;
}
EDGES.forEach((e, i) => {
  const srcDom = NODE_DOMAIN_BY_ID.get(e.source) || 'examples';
  const tgtDom = NODE_DOMAIN_BY_ID.get(e.target) || 'examples';
  cyElements.push({
    data: {
      id: `e${i}`,
      source: e.source,
      target: e.target,
      kind: e.kind,
      anomaly: e.anomaly,
      types: e.types,
      risk: e.risk,
      optional: e.optional,
      gradStops: buildEdgeGradient(srcDom, tgtDom),
      arrowColor: domainColor[tgtDom] || '#6e7681',
    },
    classes: `edge kind-${e.kind} risk-${e.risk}${e.runtime ? ' runtime' : ''}`,
  });
});

// ============================================================
// Initialise Cytoscape
// ============================================================
const cy = cytoscape({
  container: document.getElementById('cy'),
  elements: cyElements,
  wheelSensitivity: 0.2,
  minZoom: 0.15,
  maxZoom: 3.0,
  style: [
    {
      // Apply to leaf nodes only — compound parents are styled separately
      selector: 'node[size]',
      style: {
        'background-color': 'data(_color)',
        'background-opacity': 1.0,
        'label': 'data(label)',
        'color': '#c9d1d9',
        'font-size': 'data(font)',
        'font-family': 'ui-monospace, "SF Mono", Consolas, monospace',
        'text-valign': 'center',
        'text-halign': 'center',
        'text-outline-width': 2,
        'text-outline-color': '#0d1117',
        'text-outline-opacity': 0.85,
        'text-wrap': 'wrap',
        'text-max-width': '120px',
        'width': 'data(size)',
        'height': 'data(size)',
        'border-width': 2,
        'border-color': '#0d1117',
        'border-opacity': 0.9,
        'overlay-opacity': 0,
      }
    },
    // Dormant nodes
    {
      selector: 'node.status-dormant',
      style: {
        'border-style': 'dashed',
        'background-opacity': 0.30,
        'border-color': '#f85149',
        'border-width': 3,
        'color': '#8b949e',
      }
    },
    {
      selector: 'node.status-partial',
      style: {
        'border-style': 'dashed',
        'background-opacity': 0.65,
        'border-color': '#f0883e',
        'border-width': 3,
      }
    },
    {
      selector: 'node[?hasSilent]',
      style: {
        'border-color': '#f0883e',
        'border-width': 3,
      }
    },
    {
      selector: 'node[?hasAspirational]',
      style: {
        'shape': 'round-rectangle',
      }
    },
    {
      selector: 'node[isExample]',
      style: {
        'shape': 'hexagon',
        'border-style': 'dotted',
      }
    },
    // Compound parents — match BOTH the compound flag and presence of domain
    // so we can drive color off `data(domain)` mapped via a function below.
    {
      selector: 'node:parent',
      style: {
        'background-color': '#21262d',
        'background-opacity': 0.18,
        'border-color': '#30363d',
        'border-width': 1,
        'border-style': 'dotted',
        'label': 'data(label)',
        'text-valign': 'top',
        'text-halign': 'center',
        'font-size': 12,
        'font-weight': 'bold',
        'color': '#8b949e',
        'padding': 18,
        'text-outline-width': 3,
        'text-outline-color': '#0d1117',
        'shape': 'round-rectangle',
      }
    },
    // v2.1: Edges default to a source-domain → target-domain linear gradient.
    // Gradient stops + arrow color are precomputed onto edge data
    // (`data(gradStops)` / `data(arrowColor)`) to avoid per-render function
    // mappers that regressed initial-render time.
    {
      selector: 'edge',
      style: {
        'curve-style': 'bezier',   // gradients require bezier / straight / unbundled-bezier
        'width': 1.3,
        'line-fill': 'linear-gradient',
        'line-gradient-stop-colors': 'data(gradStops)',
        'line-gradient-stop-positions': '0 100',
        'line-opacity': 0.65,
        'target-arrow-shape': 'triangle',
        'target-arrow-color': 'data(arrowColor)',
        'arrow-scale': 0.85,
      }
    },
    // Risk-flagged edges revert to solid color — the safety signal overrides
    // the cross-domain visual signal.
    {
      selector: 'edge.risk-HIGH',
      style: {
        'width': 2.2,
        'line-fill': 'solid',
        'line-color': '#f0883e',
        'target-arrow-color': '#f0883e',
        'line-opacity': 0.85,
      }
    },
    {
      selector: 'edge.risk-CRITICAL',
      style: {
        'width': 2.8,
        'line-fill': 'solid',
        'line-color': '#f85149',
        'target-arrow-color': '#f85149',
        'line-opacity': 0.9,
      }
    },
    {
      selector: 'edge.kind-anomalous',
      style: {
        'line-fill': 'solid',
        'line-color': '#f85149',
        'target-arrow-color': '#f85149',
        'width': 2.5,
        'line-opacity': 0.9,
      }
    },
    {
      selector: 'edge.kind-declared-unused',
      style: {
        'line-fill': 'solid',
        'line-style': 'dashed',
        'line-color': '#8b949e',
        'target-arrow-color': '#8b949e',
        'line-opacity': 0.6,
        'width': 1.1,
        'target-arrow-shape': 'tee',
      }
    },
    {
      // v1.1: asserted runtime-integration edges (from trace front-matter `runtime_edges`).
      // Distinct from a plain cargo dependency: confirmed runtime wiring.
      selector: 'edge.runtime',
      style: {
        'line-fill': 'solid',
        'line-color': '#2dd4bf',
        'target-arrow-color': '#2dd4bf',
        'line-opacity': 0.95,
        'width': 3.0,
        'target-arrow-shape': 'triangle',
        'z-index': 20,
      }
    },
    // Selected
    {
      selector: 'node:selected',
      style: {
        'border-color': '#58a6ff',
        'border-width': 4,
        'border-style': 'solid',
      }
    },
    // v2.1: selected edge keeps its gradient — distinguish via thicker stroke
    // + shadow glow only.
    {
      selector: 'edge:selected',
      style: {
        'width': 3.5,
        'line-opacity': 1.0,
        'shadow-blur': 10,
        'shadow-color': '#58a6ff',
        'shadow-opacity': 0.7,
        'z-index': 90,
      }
    },
    // Dimmed
    {
      selector: '.dimmed',
      style: {
        'opacity': 0.10,
        'text-opacity': 0,
      }
    },
    {
      selector: '.highlighted',
      style: {
        'opacity': 1.0,
        'border-color': '#58a6ff',
        'border-width': 4,
      }
    },
    {
      selector: 'edge.highlighted',
      style: {
        'line-color': '#58a6ff',
        'target-arrow-color': '#58a6ff',
        'width': 3.5,
        'line-opacity': 1.0,
      }
    },
    {
      selector: '.path-edge',
      style: {
        'line-color': '#3fb950',
        'target-arrow-color': '#3fb950',
        'width': 4,
        'line-opacity': 1.0,
        'z-index': 999,
      }
    },
    {
      selector: '.path-node',
      style: {
        'border-color': '#3fb950',
        'border-width': 5,
        'border-style': 'solid',
        'opacity': 1.0,
      }
    },
    // Hidden by filter
    {
      selector: '.hidden',
      style: { 'display': 'none' }
    },
  ],
  layout: { name: 'preset' }, // start with no layout, apply below
});

// Overlay icons drawn after layout via DOM hack? Use compound trick instead.
// Cytoscape doesn't easily support node-overlays; instead we use color + border
// styling above. The icons appear in tooltips/panel instead.

// ============================================================
// Layouts
// ============================================================
const LAYOUTS = {
  fcose: {
    name: 'fcose',
    quality: 'proof',
    randomize: true,
    animate: true,
    animationDuration: 800,
    nodeRepulsion: 9000,
    idealEdgeLength: 110,
    nodeSeparation: 80,
    edgeElasticity: 0.45,
    gravity: 0.25,
    gravityRangeCompound: 1.5,
    gravityCompound: 1.0,
    numIter: 2500,
    nestingFactor: 0.18,
    tilingPaddingVertical: 14,
    tilingPaddingHorizontal: 14,
    packComponents: true,
  },
  dagre: {
    name: 'dagre',
    // BT draws dependency targets (the crates an edge points at) toward the top.
    rankDir: 'BT',
    nodeSep: 24,
    rankSep: 80,
    edgeSep: 20,
    animate: true,
    animationDuration: 800,
    // dagre cannot handle compound parents; runLayout detaches them first
  },
};

// Compound domain boxes stay on for fcose. Layered, dagre, and dormancy detach
// them. After detach, crate nodes are no longer `:child`, so every feature
// that means "a crate" uses crateNodes() rather than that selector.
let compoundsAttached = true;
const childParentMap = new Map();
NODES.forEach(n => childParentMap.set(n.id, `dom-${n.domain}`));

function cmpStr(a, b) {
  // Code-unit order. Independent of locale and of the order nodes were added.
  return a < b ? -1 : a > b ? 1 : 0;
}
function isDomainNode(n) { return n.data('isCompound') === true; }
function crateNodes() { return cy.nodes().filter(n => !isDomainNode(n)); }

function detachCompounds() {
  if (!compoundsAttached) return;
  crateNodes().forEach(n => n.move({ parent: null }));
  compoundsAttached = false;
  cy.nodes().filter(isDomainNode).addClass('hidden');
}
function attachCompounds() {
  if (compoundsAttached) return;
  childParentMap.forEach((p, id) => {
    const n = cy.getElementById(id);
    if (n.length) n.move({ parent: p });
  });
  compoundsAttached = true;
}

function runLayout(name) {
  cy.stop();
  currentLayoutName = name;
  if (name === 'dormancy') {
    detachCompounds();
    runDormancyStratification();
  } else   if (name === 'dagre') {
    detachCompounds();
    cy.layout(Object.assign({}, LAYOUTS.dagre, {
      eles: crateNodes().union(cy.edges()),
    })).run();
  } else if (name === 'layered') {
    layoutLayered();
  } else {
    attachCompounds();
    runClusterLayout(name, false);
  }
  applyFilters();
}

// fcose is registered by the vendored extension. If that registration is
// missing, Domain clusters uses cytoscape's built-in cose layout instead of
// throwing.
function layoutRegistered(name) {
  try {
    return typeof cytoscape === 'function' && !!cytoscape('layout', name);
  } catch (err) {
    return false;
  }
}
function runClusterLayout(name, randomize) {
  var useFcose = (name === 'fcose' || !LAYOUTS[name]) && layoutRegistered('fcose');
  if (name !== 'fcose' && LAYOUTS[name]) useFcose = false;
  if (useFcose) {
    var opt = Object.assign({}, LAYOUTS.fcose);
    if (randomize) opt.randomize = true;
    cy.layout(opt).run();
    return;
  }
  if ((name === 'fcose' || !LAYOUTS[name]) && layoutRegistered('cose')) {
    cy.layout({
      name: 'cose',
      animate: true,
      animationDuration: 600,
      idealEdgeLength: 110,
      nodeRepulsion: 8000,
      padding: 30,
    }).run();
    return;
  }
  cy.layout(LAYOUTS[name] || { name: 'circle', padding: 30 }).run();
}

// Tier rule (also in the How to use note):
//   Edge A → B means A depends on B.
//   dagre ranker `longest-path`, rankdir `BT`.
//   Nodes are set in id order. Edges are set in (source, target, id) order.
//   dagre 0.8.5 has no Math.random. acyclicer is unset, so acyclic.run uses
//   dfsFAS (a depth-first cycle break), which walks nodes and out-edges in
//   insertion order. The sorts above make that walk deterministic.
//   Tier 0 is the top row (no workspace dependency). Deeper tiers are lower.
//   How many edges point up or down is counted from these positions
//   (publishEdgeFlow). It is not a fixed number.
function layeredNodeBox(n) {
  const s = n.data('size') || 28;
  const label = String(n.data('label') || n.id());
  return {
    w: Math.max(s + 36, Math.min(140, Math.ceil(label.length * 6.2) + 12)),
    h: Math.max(s, 32) + 10,
  };
}
function layoutLayered() {
  detachCompounds();
  const crates = crateNodes().toArray().sort((a, b) => cmpStr(a.id(), b.id()));
  if (typeof dagre === 'undefined' || !dagre.graphlib) {
    layoutLayeredFallback(crates);
    return;
  }
  const g = new dagre.graphlib.Graph({ multigraph: true, compound: false });
  g.setGraph({
    rankdir: 'BT',
    nodesep: 12,
    ranksep: 110,
    edgesep: 6,
    ranker: 'longest-path',
    marginx: 16,
    marginy: 16,
  });
  g.setDefaultEdgeLabel(() => ({}));
  crates.forEach(n => {
    const box = layeredNodeBox(n);
    g.setNode(n.id(), { width: box.w, height: box.h });
  });
  const edges = cy.edges().toArray().filter(e => {
    return g.hasNode(e.data('source')) && g.hasNode(e.data('target'));
  }).sort((a, b) => cmpStr(
    a.data('source') + '\0' + a.data('target') + '\0' + a.id(),
    b.data('source') + '\0' + b.data('target') + '\0' + b.id()
  ));
  edges.forEach(e => {
    g.setEdge(e.data('source'), e.data('target'), { minlen: 1, weight: 1 }, e.id());
  });
  try {
    dagre.layout(g);
  } catch (err) {
    console.error('layered layout failed; using the tier fallback', err);
    layoutLayeredFallback(crates);
    return;
  }
  const positions = {};
  crates.forEach(n => {
    const p = g.node(n.id());
    positions[n.id()] = { x: p.x, y: p.y };
  });
  applyPresetPositions(positions);
  publishEdgeFlow();
}
// Same tiers if dagre itself failed to load. X order inside a tier is id order.
function layoutLayeredFallback(crates) {
  const outgoingIds = new Map();
  crates.forEach(n => outgoingIds.set(n.id(), []));
  cy.edges().forEach(e => {
    const s = e.data('source');
    const t = e.data('target');
    if (outgoingIds.has(s) && outgoingIds.has(t)) outgoingIds.get(s).push(t);
  });
  outgoingIds.forEach(list => list.sort(cmpStr));
  const rank = new Map();
  const stack = new Set();
  function dfs(u) {
    if (rank.has(u)) return rank.get(u);
    if (stack.has(u)) return null;
    stack.add(u);
    let best = 0;
    for (const v of outgoingIds.get(u) || []) {
      const rv = dfs(v);
      if (rv === null) continue;
      best = Math.max(best, rv + 1);
    }
    stack.delete(u);
    const value = (outgoingIds.get(u) || []).length ? best : 0;
    rank.set(u, value);
    return value;
  }
  crates.forEach(n => dfs(n.id()));
  const rows = new Map();
  crates.forEach(n => {
    const r = rank.get(n.id()) || 0;
    if (!rows.has(r)) rows.set(r, []);
    rows.get(r).push(n);
  });
  const positions = {};
  [...rows.keys()].sort((a, b) => a - b).forEach(r => {
    const row = rows.get(r).sort((a, b) => cmpStr(a.id(), b.id()));
    row.forEach((n, i) => {
      const box = layeredNodeBox(n);
      positions[n.id()] = { x: i * (box.w + 16), y: r * 90 };
    });
  });
  applyPresetPositions(positions);
  publishEdgeFlow();
}
// Count arrow direction from the drawn tier positions. An edge points upward
// when its target sits on a higher row (smaller y) than its source. The ids
// are read from the graph, so a later edge change does not leave a stale count.
function publishEdgeFlow() {
  var el = document.getElementById('edge-flow');
  if (!el) return;
  var y = {};
  crateNodes().forEach(function (n) { y[n.id()] = n.position().y; });
  var up = 0;
  var down = 0;
  var flat = 0;
  var downIds = [];
  cy.edges().forEach(function (e) {
    var s = e.data('source');
    var t = e.data('target');
    if (!(s in y) || !(t in y)) return;
    if (y[s] > y[t] + 0.5) up++;
    else if (y[t] > y[s] + 0.5) {
      down++;
      downIds.push(s + ' → ' + t);
    } else flat++;
  });
  downIds.sort(cmpStr);
  var total = up + down + flat;
  var text = up + ' of ' + total + ' edges point upward.';
  if (downIds.length) {
    text += ' Downward: ' + downIds.join(', ') + '.';
    var veilOnly = downIds.every(function (id) { return id.indexOf('astraweave-veil → ') === 0; });
    if (veilOnly) {
      text += ' These are runtime edges declared in veil.md. Review #733 E3 reports them as reversed, not as dependency cycles. They drop off this list when that reversal is fixed.';
    } else {
      text += ' A downward edge is not a dependency cycle.';
    }
  } else {
    text += ' No edge points downward.';
  }
  if (flat) text += ' Same row: ' + flat + '.';
  el.textContent = text;
}
function applyPresetPositions(positions) {
  cy.layout({
    name: 'preset',
    animate: false,
    fit: true,
    padding: 36,
    eles: crateNodes(),
    positions: (node) => positions[node.id()] || node.position(),
  }).run();
}

function runDormancyStratification() {
  // Custom radial: active core in center, partial mid ring, dormant outer ring.
  // Id order inside a ring, so the rings do not depend on element insertion order.
  const center = { x: 0, y: 0 };
  const bands = { active: 380, partial: 780, dormant: 1180 };
  const groups = { active: [], partial: [], dormant: [] };
  crateNodes().toArray().sort((a, b) => cmpStr(a.id(), b.id())).forEach(n => {
    const s = n.data('status') || 'active';
    if (!groups[s]) groups.active.push(n); else groups[s].push(n);
  });
  const positions = {};
  Object.entries(groups).forEach(([k, arr]) => {
    const r = bands[k];
    const N = arr.length || 1;
    arr.forEach((n, i) => {
      const theta = (i / N) * 2 * Math.PI - Math.PI / 2;
      positions[n.id()] = {
        x: center.x + r * Math.cos(theta),
        y: center.y + r * Math.sin(theta),
      };
    });
  });
  applyPresetPositions(positions);
}

document.getElementById('layout-select').addEventListener('change', (e) => {
  runLayout(e.target.value);
});

// --------------------------------------------------------------------
// Initial layout
//   Default is the deterministic layered drawing. A hash can still select
//   fcose, dagre, or dormancy. Wait until the container has a size so the
//   fit is not computed against a 0×0 flex box.
// --------------------------------------------------------------------
function waitForContainerSize(el, cb, attempts = 30) {
  const r = el.getBoundingClientRect();
  if (r.width > 50 && r.height > 50) { cb(); return; }
  if (attempts <= 0) { cb(); return; }
  requestAnimationFrame(() => waitForContainerSize(el, cb, attempts - 1));
}

let currentLayoutName = 'layered';
let bootLayoutStarted = false;

function runInitialLayout() {
  if (bootLayoutStarted) return;
  bootLayoutStarted = true;
  const sel = document.getElementById('layout-select');
  const hash = decodeHash();
  let chosen = 'layered';
  if (hash.layout && [...sel.options].some(o => o.value === hash.layout)) chosen = hash.layout;
  sel.value = chosen;
  cy.resize();
  runLayout(chosen);
}

// The initial layout is started at the bottom of this script, after the
// filter controls it refreshes exist.

// Window resize: refit so nodes don't fall off-screen when the toolbar
// re-wraps or the user resizes the side panel.
let resizeTimer = null;
window.addEventListener('resize', () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    cy.resize();
    cy.fit(undefined, 40);
  }, 120);
});

// Re-explode button: re-run the current layout from scratch with fresh
// randomisation so the user can shake the graph out if it ended up packed.
document.getElementById('re-explode').addEventListener('click', () => {
  const sel = document.getElementById('layout-select').value;
  if (sel === 'fcose') {
    attachCompounds();
    currentLayoutName = 'fcose';
    runClusterLayout('fcose', true);
    applyFilters();
  } else {
    runLayout(sel);
  }
});

// Layout selector: also update currentLayoutName so Re-explode knows what to do
document.getElementById('layout-select').addEventListener('change', (e) => {
  currentLayoutName = e.target.value;
});

// ============================================================
// Right side panel
// ============================================================
const sidePanel = document.getElementById('side-panel');
const panelContent = document.getElementById('panel-content');
const panelToggle = document.getElementById('panel-toggle');
panelToggle.addEventListener('click', () => {
  const collapsed = sidePanel.classList.toggle('collapsed');
  panelToggle.textContent = collapsed ? '‹ Open' : 'Hide ›';
});

function escapeHtml(s) {
  if (s == null) return '';
  return String(s)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

function traceLink(trace, section) {
  if (!trace) return '';
  const href = section ? `${TRACE_BASE}${trace}${section}` : `${TRACE_BASE}${trace}`;
  return `<a href="${escapeHtml(href)}" target="_blank" rel="noopener">${escapeHtml(trace)}${section ? ' '+escapeHtml(section) : ''}</a>`;
}

// One-line summaries (Trace-level short descriptions per ARCHITECTURE_MAP §0)
const ONE_LINERS = {
  'astraweave-render':         'Engine renderer. 78K LoC, 71 WGSL shaders. Canonical 32-layer terrain pipeline. Editor depends on this since Fix-27.',
  'astraweave-physics':        'Rapier3D wrapper. PhysicsWorld + CharacterController. SpatialHash module dormant (1,038 LoC) despite doc-comment.',
  'astraweave-fluids':         'Dormant. 84.5K LoC. Only examples/fluids_demo consumes it. Five parallel solver/manager surfaces, no game-loop deps.',
  'astraweave-terrain':        '30+ modules including climate field, Whittaker biome lookup, regional archetype variation. Reverse-dep on gameplay (BiomeType).',
  'astraweave-ecs':            'Archetype substrate. Generational Entity {id, generation}. Deterministic single-threaded scheduler (ParallelSchedule removed 2026-04-18).',
  'astraweave-core':           'Foundation types: WorldSnapshot, PlanIntent, ActionStep. Plus the LEGACY core::World (HashMap-per-component) bridged by build_app.',
  'astraweave-math':           'SIMD batch ops on Vec3/Vec4/Mat4/Quat. 1.7-2.5× over scalar glam per Week 5-6 measurements.',
  'astraweave-sdk':            'C ABI for embedding. GameAdapter trait. One in-tree consumer (examples/sdk_c_harness).',
  'astraweave-ai':             'AIArbiter + orchestrators. AI-first commitment. Runtime LLM defaults to phi3:medium despite Qwen3 doc-comments.',
  'astraweave-behavior':       'Canonical GOAP + BehaviorNode enum. WorldState is BTreeMap<u32, bool> with interned keys.',
  'astraweave-llm':            'LLM client adapters (Phi3, Hermes2Pro, Qwen3). Hardening surface (~15K LoC) shelf-stocked but bypassed at runtime.',
  'astraweave-memory':         'Dormant. ~11K LoC. Zero in-engine production consumers; only persona uses legacy persona::* types.',
  'astraweave-coordination':   'Dormant. ~5.3K LoC. Zero workspace consumers. 3 commented-out pub mod declarations whose source files never existed.',
  'astraweave-rag':            'Dormant composite. Held as field in 5 dormant LLM-enhanced consumer crates. Advertised HNSW is linear scan.',
  'astraweave-dialogue':       'Mixed. Basic DialogueGraph/DialogueRunner production-wired; llm_dialogue.rs (60% of crate) dormant.',
  'astraweave-net':            'Snapshot-based server. IVec2 grid. JSON over WebSocket. Disjoint from astraweave-net-ecs.',
  'astraweave-net-ecs':        'Matchmaking ECS layer. Plugin layer is dormant. Wire format mismatch with standalone server.',
  'astraweave-persistence-ecs':'A health write is a little-endian i32 into an existing CHealth on the CReplayState entity. A later save error leaves earlier managers saved.',
  'astraweave-input':          'Pure facade over winit + gilrs. Declared-but-unused by gameplay/ui. Editor reinvents in 2,511-LoC panel.',
  'astraweave-audio':          'rodio facade. 5 buses (master/music/ambient/voice/SFX). !Send+!Sync so cannot be ECS Resource.',
  'astraweave-scene':          'Transform, Node, Scene + animation ECS components. compute_poses_stub is documented stub.',
  'astraweave-nav':            'A* pathfinding, navmesh baking, dirty region tracking.',
  'astraweave-gameplay':       '20+ modules: combat, crafting, dialogue, quests, items, stats, biome, harvesting, weaving.',
  'aw_editor':                 '224K LoC visual editor. EditorApp god struct (123 fields). §7.7 trap surfaced at 4 layers; Real-Fix.D pending.',
  'examples_collapsed':        '59 example crates collapsed into one node. Notable: hello_companion, unified_showcase, fluids_demo.',
};

function renderNodePanel(n) {
  const status = n.status || 'active';
  const statusLabel = status === 'active' ? 'ACTIVE'
                    : status === 'dormant' ? 'DORMANT'
                    : status === 'partial' ? 'PARTIAL'
                    : status.toUpperCase();
  const oneLiner = ONE_LINERS[n.id] || (n.statusEvidence ? n.statusEvidence.split('.')[0] + '.' : '(no summary — see ARCHITECTURE_MAP.md)');

  const inEdges = incoming.get(n.id) || [];
  const outEdges = outgoing.get(n.id) || [];

  function edgeLine(e, isIn) {
    const peer = isIn ? e.source : e.target;
    const arrow = isIn ? '← ' : '→ ';
    const risk = riskPill(e.risk);
    const kindMark = e.kind === 'anomalous' ? ' <span style="color:var(--danger);font-weight:bold;" title="Anomalous">⚠</span>'
                   : e.kind === 'declared-unused' ? ' <span style="color:var(--text-2);" title="Declared but unused">✕</span>'
                   : '';
    return `<li><code class="inline">${arrow}${escapeHtml(peer)}</code> ${risk}${kindMark}</li>`;
  }

  // Primary actions row
  const traceBtn = n.trace
    ? `<a class="btn-sm" href="${TRACE_BASE}${escapeHtml(n.trace)}" target="_blank" rel="noopener">Open trace ↗</a>`
    : `<button class="btn-sm" disabled title="No dedicated trace exists">No trace ✕</button>`;
  const focusActive = state.focusModeActive ? 'on' : '';
  const blastActive = state.blastEnabled ? 'on' : '';
  const actionsRow = `
    <div class="panel-actions">
      ${traceBtn}
      <button class="btn-sm ${blastActive}" id="panel-blast"
        title="Highlight transitive dependents up to 3 hops">${state.blastEnabled ? '◉' : '○'} Blast radius</button>
      <button class="btn-sm ${focusActive}" id="panel-focus"
        title="Hide nodes outside 1-hop neighborhood">${state.focusModeActive ? '◉' : '○'} Focus 1-hop</button>
      <button class="btn-sm" id="panel-center"
        title="Center and zoom on this node">⊕ Highlight in graph</button>
    </div>`;

  // Blast radius counts (if any dependents exist)
  let blastSummary = '';
  if (inEdges.length > 0) {
    const radius = computeBlastRadius(n.id);
    const N = radius.direct.size;
    const M = radius.hop2.size + radius.hop3.size;
    blastSummary = `
      <div class="panel-section" style="margin-bottom:10px;">
        <h3>Affected crates if this changes</h3>
        <div style="font-size:12px;color:var(--text-1);line-height:1.55;">
          <strong style="color:var(--warn);">${N} direct</strong>${M > 0 ? `, <strong>${M} transitive</strong> (within 3 hops)` : ''}. ${N === 0 ? 'No crate currently depends on this.' : 'This is dependency-graph highlighting, not behavioral simulation.'}
        </div>
        ${N > 0 ? `<details style="margin-top:8px;"><summary>Direct dependents <span class="count">(${N})</span></summary>
          <div class="details-body"><ul class="compact">
            ${Array.from(radius.direct).sort().map(id => `<li><code class="inline">${escapeHtml(id)}</code></li>`).join('')}
          </ul></div></details>` : ''}
      </div>`;
  }

  // Trace section
  const traceDeepLinks = n.trace ? `
    <div class="details-body">
      <div style="font-size:11px;line-height:1.7;">
        <a href="${TRACE_BASE}${escapeHtml(n.trace)}#5-active-file-map" target="_blank" rel="noopener">§5 File Map</a> ·
        <a href="${TRACE_BASE}${escapeHtml(n.trace)}#6-conflict-map" target="_blank" rel="noopener">§6 Conflicts</a> ·
        <a href="${TRACE_BASE}${escapeHtml(n.trace)}#7-decision-log" target="_blank" rel="noopener">§7 Decisions</a> ·
        <a href="${TRACE_BASE}${escapeHtml(n.trace)}#8-invariants" target="_blank" rel="noopener">§8 Invariants</a> ·
        <a href="${TRACE_BASE}${escapeHtml(n.trace)}#11-open-questions" target="_blank" rel="noopener">§11 Open Questions</a>
      </div>
    </div>` : '';

  const evidenceBody = n.statusEvidence ? `
      <details><summary>Status evidence <span class="count">${escapeHtml(status)}</span></summary>
        <div class="details-body" style="color:var(--text-1);">${escapeHtml(n.statusEvidence)}</div>
      </details>` : '';

  const aspirationalBody = n.aspirationalDocs ? `
      <details><summary>📜 Aspirational documentation drift <span class="count">§7.1</span></summary>
        <div class="details-body" style="color:var(--text-1);">${escapeHtml(n.aspirationalDocs)}</div>
      </details>` : '';

  const depsBody = `
    <details>
      <summary>Dependencies <span class="count">${inEdges.length} ↑ in · ${outEdges.length} ↓ out</span></summary>
      <div class="details-body">
        ${inEdges.length ? `<div style="margin:4px 0 8px 0;"><strong style="font-size:11px;color:var(--text-1);text-transform:uppercase;letter-spacing:0.5px;">Used by (${inEdges.length})</strong>
          <ul class="compact" style="margin-top:4px;">${inEdges.map(e => edgeLine(e, true)).join('')}</ul></div>` : ''}
        ${outEdges.length ? `<div><strong style="font-size:11px;color:var(--text-1);text-transform:uppercase;letter-spacing:0.5px;">Depends on (${outEdges.length})</strong>
          <ul class="compact" style="margin-top:4px;">${outEdges.map(e => edgeLine(e, false)).join('')}</ul></div>` : ''}
        ${!inEdges.length && !outEdges.length ? '<div style="font-size:12px;color:var(--text-2);font-style:italic;">No declared dependencies in either direction.</div>' : ''}
      </div>
    </details>`;

  // Integration types — show if this crate is a source-of-shared-types per §6.1
  const sharedTypeRows = [];
  outEdges.forEach(e => { if (e.types && e.types.length && e.risk !== 'LOW') sharedTypeRows.push(e); });
  inEdges.forEach(e => { if (e.types && e.types.length && e.risk !== 'LOW') sharedTypeRows.push(e); });
  const intTypesBody = sharedTypeRows.length ? `
    <details>
      <summary>Integration types <span class="count">§6.1 · ${sharedTypeRows.length} seam${sharedTypeRows.length === 1 ? '' : 's'}</span></summary>
      <div class="details-body">
        <ul class="compact">${sharedTypeRows.map(e => {
          const dir = e.source === n.id ? '→' : '←';
          const peer = e.source === n.id ? e.target : e.source;
          return `<li><code class="inline">${dir} ${escapeHtml(peer)}</code> ${riskPill(e.risk)}<br>
            <span style="font-size:11px;color:var(--text-1);">${e.types.map(escapeHtml).join(', ')}</span></li>`;
        }).join('')}</ul>
      </div>
    </details>` : '';

  // Open questions — only if §14 names this crate
  const qBody = n.openQuestions && n.openQuestions.length ? `
    <details>
      <summary>Open questions <span class="count">§14 · ${n.openQuestions.length}</span></summary>
      <div class="details-body"><ul class="compact">${n.openQuestions.map(q => `<li>${escapeHtml(q)}</li>`).join('')}</ul></div>
    </details>` : '';

  // Silent failures — only if §4.3 names this crate
  const silentBody = n.silentFailures && n.silentFailures.length ? `
    <details>
      <summary>⚠ Silent-failure shapes <span class="count">§4.3 · ${n.silentFailures.length}</span></summary>
      <div class="details-body"><ul class="compact">${n.silentFailures.map(s => `<li>${escapeHtml(s)}</li>`).join('')}</ul></div>
    </details>` : '';

  const exampleBody = n.isExample && n.exampleList ? `
    <details>
      <summary>Collapsed examples <span class="count">${n.exampleList.length} crates</span></summary>
      <div class="details-body" style="max-height:240px;overflow-y:auto;font-family:ui-monospace,monospace;font-size:11px;color:var(--text-1);">
        ${n.exampleList.map(name => `<div>${escapeHtml(name)}</div>`).join('')}
      </div>
    </details>` : '';

  const traceBody = n.trace ? `
    <details>
      <summary>Trace deep-links <span class="count">${escapeHtml(n.trace)}</span></summary>
      ${traceDeepLinks}
    </details>` : '';

  panelContent.innerHTML = `
    <div class="panel-section" style="margin-bottom:6px;">
      <h3>Crate</h3>
      <h2>${escapeHtml(n.label)}</h2>
      <div class="meta-grid">
        <span class="key">Domain</span><span class="val"><span class="badge-dot" style="background:${escapeHtml(domainColor[n.domain] || '#888')}"></span>${escapeHtml(n.domainLabel)}</span>
        <span class="key">Status</span><span class="val"><span class="status-pill ${escapeHtml(status)}">${escapeHtml(statusLabel)}</span></span>
        <span class="key">LoC band</span><span class="val">${escapeHtml(n.locBand)}${n.locApprox ? ' (~'+n.locApprox.toLocaleString()+')' : ''}</span>
      </div>
      <div class="one-liner">${escapeHtml(oneLiner)}</div>
    </div>
    ${actionsRow}
    ${blastSummary}
    ${traceBody}
    ${depsBody}
    ${intTypesBody}
    ${qBody}
    ${silentBody}
    ${evidenceBody}
    ${aspirationalBody}
    ${exampleBody}
  `;

  // Wire up the panel's primary-action buttons
  const panelBlast = document.getElementById('panel-blast');
  const panelFocus = document.getElementById('panel-focus');
  const panelCenter = document.getElementById('panel-center');
  if (panelBlast) panelBlast.addEventListener('click', () => toggleBlast());
  if (panelFocus) panelFocus.addEventListener('click', () => toggleFocus());
  if (panelCenter) panelCenter.addEventListener('click', () => centerOnNode(n.id));
}

function riskPill(risk) {
  const safe = escapeHtml(risk);
  return `<span class="risk-pill ${safe}">${safe}</span>`;
}

function renderEdgePanel(e) {
  const risk = riskPill(e.risk);
  const types = e.types && e.types.length ? `
    <div class="panel-section">
      <h3>Load-bearing types flowing across</h3>
      <ul class="compact">${e.types.map(t => `<li><code class="inline">${escapeHtml(t)}</code></li>`).join('')}</ul>
    </div>` : `
    <div class="panel-section">
      <h3>Load-bearing types</h3>
      <div class="empty-panel" style="text-align:left;padding:0;font-size:12px;">Not documented in ARCHITECTURE_MAP.md §6.1.</div>
    </div>`;
  const anomalyBlock = e.anomaly ? `
    <div class="panel-section">
      <h3>⚠ Anomaly note (§2.3)</h3>
      <div style="font-size:12px;color:var(--warn);">${escapeHtml(e.anomaly)}</div>
    </div>` : '';
  const kindLabel = e.kind === 'anomalous' ? 'ANOMALOUS'
                  : e.kind === 'declared-unused' ? 'DECLARED-BUT-UNUSED'
                  : 'WORKSPACE-DEP';
  panelContent.innerHTML = `
    <div class="panel-section">
      <h3>Dependency edge</h3>
      <h2 style="font-size:14px;"><code class="inline">${escapeHtml(e.source)}</code> → <code class="inline">${escapeHtml(e.target)}</code></h2>
      <div class="meta-row"><span class="key">Kind</span><span class="val">${escapeHtml(kindLabel)}</span></div>
      <div class="meta-row"><span class="key">Risk</span><span class="val">${risk}</span></div>
      <div class="meta-row"><span class="key">Optional</span><span class="val">${e.optional ? 'yes' : 'no'}</span></div>
    </div>
    ${types}
    ${anomalyBlock}
  `;
}

cy.on('tap', 'node', (evt) => {
  const target = evt.target;
  if (target.isParent()) return;
  // Path-pick mode handling
  if (pickMode === 'A') {
    pathA = target.id();
    document.getElementById('path-a').textContent = pathA;
    setPickMode(null);
    updatePathBtn();
    return;
  }
  if (pickMode === 'B') {
    pathB = target.id();
    document.getElementById('path-b').textContent = pathB;
    setPickMode(null);
    updatePathBtn();
    return;
  }
  const nodeData = target.data();
  renderNodePanel({
    id: nodeData.id,
    label: nodeData.label,
    domain: nodeData.domain,
    domainLabel: nodeData.domainLabel,
    kind: nodeData.isExample ? 'example' : 'production',
    locBand: nodeData.locBand,
    locApprox: nodeData.locApprox,
    status: nodeData.status,
    statusEvidence: nodeData.statusEvidence,
    trace: nodeData.trace,
    silentFailures: nodeData.silentFailures || [],
    aspirationalDocs: nodeData.aspirationalDocs,
    openQuestions: nodeData.openQuestions || [],
    isExample: nodeData.isExample,
    exampleList: nodeData.exampleList,
  });
});

cy.on('tap', 'edge', (evt) => {
  const e = evt.target.data();
  renderEdgePanel(e);
});

cy.on('tap', (evt) => {
  if (evt.target === cy) {
    // background click — clear path-pick
    setPickMode(null);
  }
});

// ============================================================
// Hover tooltips
// ============================================================
const tooltip = document.getElementById('cy-tooltip');
let lastHover = null;

cy.on('mouseover', 'node', (evt) => {
  const n = evt.target;
  if (n.isParent()) return;
  const d = n.data();
  const status = d.status || 'active';
  const statusLabel = status === 'active' ? 'active'
                    : status === 'dormant' ? 'dormant'
                    : status === 'partial' ? 'partial'
                    : status;
  const overlays = [];
  if (d.hasSilent) overlays.push('⚠ silent-failure locus');
  if (d.hasTrace) overlays.push('📄 traced');
  if (d.hasAspirational) overlays.push('📜 aspirational docs');
  tooltip.innerHTML = `
    <div class="tip-title">${escapeHtml(d.label)}</div>
    <div class="tip-row"><span class="v">${escapeHtml(d.domainLabel || '')}</span></div>
    <div class="tip-row">LoC band: <span class="v">${escapeHtml(d.locBand)}${d.locApprox ? ' (~'+d.locApprox.toLocaleString()+')' : ''}</span></div>
    <div class="tip-row">Status: <span class="v">${escapeHtml(statusLabel)}</span></div>
    ${overlays.length ? '<div class="tip-row"><span class="v">'+overlays.join(' · ')+'</span></div>' : ''}
    <div class="tip-row" style="margin-top:3px;font-size:10px;opacity:0.6;">Click to inspect</div>
  `;
  tooltip.classList.add('visible');
});

cy.on('mouseover', 'edge', (evt) => {
  const d = evt.target.data();
  const typesLine = d.types && d.types.length
    ? d.types.slice(0, 3).map(escapeHtml).join(', ')
    : '<em style="color:var(--text-2);">no documented types</em>';
  const anomalyLine = d.anomaly ? `<div class="tip-row" style="color:var(--warn);">⚠ ${escapeHtml(d.anomaly.slice(0,120))}…</div>` : '';
  tooltip.innerHTML = `
    <div class="tip-title">${escapeHtml(d.source)} → ${escapeHtml(d.target)}</div>
    <div class="tip-row">Risk: <span class="v">${escapeHtml(d.risk)}</span></div>
    <div class="tip-row">Types: <span class="v">${typesLine}</span></div>
    ${anomalyLine}
  `;
  tooltip.classList.add('visible');
});

cy.on('mousemove', (evt) => {
  if (tooltip.classList.contains('visible')) {
    const x = evt.originalEvent.clientX + 14;
    const y = evt.originalEvent.clientY + 14;
    tooltip.style.left = Math.min(x, window.innerWidth - 340) + 'px';
    tooltip.style.top  = Math.min(y, window.innerHeight - 120) + 'px';
  }
});
cy.on('mouseout', 'node, edge', () => {
  tooltip.classList.remove('visible');
});

// ============================================================
// Search + dormancy filter
// ============================================================
const searchInput = document.getElementById('search');
const dormancyFilter = document.getElementById('dormancy-filter');
searchInput.addEventListener('input', applyFilters);
dormancyFilter.addEventListener('change', applyFilters);

function applyFilters() {
  const q = searchInput.value.trim().toLowerCase();
  const dorm = dormancyFilter.value;
  // v2.1 model: domain filter is only "active" when the user has picked a
  // proper subset. Empty set or full set both mean "no filter applied."
  const domainFilterActive = isDomainFilterActive();
  let visibleCount = 0;
  const totalCount = crateNodes().length;
  const visiblePerDomain = {};
  crateNodes().forEach(n => {
    const d = n.data();
    let visible = true;
    if (q && !d.label.toLowerCase().includes(q)) visible = false;
    if (domainFilterActive && !enabledDomains.has(d.domain)) visible = false;
    if (dorm === 'active' && d.status !== 'active') visible = false;
    if (dorm === 'dormant' && d.status === 'active') visible = false;
    if (dorm === 'silent' && !d.hasSilent) visible = false;
    if (visible) {
      n.removeClass('hidden');
      visibleCount++;
      visiblePerDomain[d.domain] = (visiblePerDomain[d.domain] || 0) + 1;
    } else {
      n.addClass('hidden');
    }
  });
  cy.edges().forEach(e => {
    const src = cy.getElementById(e.data('source'));
    const tgt = cy.getElementById(e.data('target'));
    if (src.hasClass('hidden') || tgt.hasClass('hidden')) e.addClass('hidden');
    else e.removeClass('hidden');
  });
  // Domain boxes are real nodes. While a layout has detached them they must
  // stay hidden, or they draw as stray dots. While attached, hide a box whose
  // crates are all filtered out.
  cy.nodes().filter(isDomainNode).forEach(p => {
    if (!compoundsAttached) { p.addClass('hidden'); return; }
    const domId = p.data('domain');
    if (!visiblePerDomain[domId]) p.addClass('hidden');
    else p.removeClass('hidden');
  });
  // Update counter pill
  const counter = document.getElementById('filter-counter');
  const filtered = visibleCount !== totalCount;
  counter.textContent = filtered
    ? `Showing ${visibleCount} of ${totalCount}  (filtered)`
    : `Showing ${visibleCount} of ${totalCount}`;
  counter.classList.toggle('filtered', filtered);
  updateFilterHint();
}

// ============================================================
// Path-pick controls
// ============================================================
let pickMode = null;
let pathA = null;
let pathB = null;
function setPickMode(mode) {
  pickMode = mode;
  document.getElementById('pick-a').classList.toggle('primary', mode === 'A');
  document.getElementById('pick-b').classList.toggle('primary', mode === 'B');
  document.getElementById('cy').classList.toggle('path-pick-active', mode !== null);
}
function updatePathBtn() {
  document.getElementById('show-path').disabled = !(pathA && pathB);
}
document.getElementById('pick-a').addEventListener('click', () => setPickMode('A'));
document.getElementById('pick-b').addEventListener('click', () => setPickMode('B'));
document.getElementById('show-path').addEventListener('click', () => {
  if (!pathA || !pathB) return;
  // Use Cytoscape's aStar to find shortest dependency path (directed)
  const a = cy.getElementById(pathA);
  const b = cy.getElementById(pathB);
  if (!a.length || !b.length) return;
  const dijk = cy.elements().dijkstra({
    root: a,
    weight: () => 1,
    directed: true,
  });
  const pathTo = dijk.pathTo(b);
  if (!pathTo || pathTo.length === 0) {
    alert(`No directed dependency path from ${pathA} to ${pathB}. Try reversing.`);
    return;
  }
  // Dim everything; highlight path
  cy.elements().addClass('dimmed');
  cy.elements().removeClass('path-edge path-node');
  pathTo.forEach(el => {
    el.removeClass('dimmed');
    el.addClass(el.isEdge() ? 'path-edge' : 'path-node');
  });
  document.getElementById('banner').classList.add('active');
  const nodes = pathTo.filter(el => el.isNode());
  const labels = nodes.map(n => n.id());
  document.getElementById('banner').innerHTML =
    `<strong>Path:</strong> ${labels.join(' → ')}  ·  ${pathTo.filter(el => el.isEdge()).length} edge(s) traversed. Hops = ${nodes.length - 1}. Click <em>Reset view</em> to clear.`;
});

// ============================================================
// Stories
// ============================================================
storySelect.addEventListener('change', (e) => applyStory(e.target.value));

// v2.1: disable/enable domain chips during a curated view so the user can't
// fight the story or tour with their own filter input. Re-enabled on
// reset, on tour exit, or when returning to 'free' story.
function setChipsEnabled(enabled) {
  document.querySelectorAll('.domain-chip').forEach(c => c.disabled = !enabled);
}

function applyStory(id) {
  cy.elements().removeClass('dimmed highlighted path-edge path-node');
  const story = STORIES.find(s => s.id === id);
  if (!story) return;
  const banner = document.getElementById('banner');
  if (id === 'free') {
    banner.classList.remove('active');
    banner.innerHTML = '';
    // v2.1: re-enable chips when returning to free exploration (unless a tour
    // is still active — the tour driver re-enables on exit).
    if (typeof state === 'undefined' || !state.currentTour) setChipsEnabled(true);
    return;
  }
  // v2.1: during a story preset, lock the filter chips.
  setChipsEnabled(false);
  banner.classList.add('active');
  banner.innerHTML = `<strong>${escapeHtml(story.title)}</strong> — ${escapeHtml(story.banner)}`;
  if (story.dimRest) cy.elements().addClass('dimmed');
  const hlNodes = new Set(story.highlightNodes || []);
  hlNodes.forEach(id => {
    const n = cy.getElementById(id);
    if (n.length) {
      n.removeClass('dimmed');
      n.addClass('highlighted');
    }
  });
  // Highlight edges between highlighted nodes
  cy.edges().forEach(e => {
    if (hlNodes.has(e.data('source')) && hlNodes.has(e.data('target'))) {
      e.removeClass('dimmed');
      e.addClass('highlighted');
    }
  });
  // Plus explicit highlight edges
  (story.highlightEdges || []).forEach(([s, t]) => {
    cy.edges().forEach(e => {
      if (e.data('source') === s && e.data('target') === t) {
        e.removeClass('dimmed');
        e.addClass('highlighted');
      }
    });
  });
  // Fit to highlighted set
  const fitTo = cy.nodes('.highlighted');
  if (fitTo.length) cy.fit(fitTo, 80);
}

// ============================================================
// Reset view
// ============================================================
document.getElementById('reset-view').addEventListener('click', () => {
  cy.elements().removeClass('dimmed highlighted path-edge path-node hidden');
  document.getElementById('banner').classList.remove('active');
  document.getElementById('banner').innerHTML = '';
  document.getElementById('path-a').textContent = '—';
  document.getElementById('path-b').textContent = '—';
  pathA = pathB = null;
  updatePathBtn();
  searchInput.value = '';
  dormancyFilter.value = 'all';
  storySelect.value = 'free';
  // v2.1: Reset domain chips back to empty (no-filter) state. All chips
  // become un-highlighted and re-enabled (in case a story disabled them).
  enabledDomains.clear();
  DOMAINS.forEach((d, i) => {
    const chip = domainFilters.querySelectorAll('.domain-chip')[i];
    chip.classList.remove('on');
    chip.setAttribute('aria-pressed', 'false');
    chip.disabled = false;
  });
  // Reset counter
  applyFilters();
  cy.fit(undefined, 40);
});

// ============================================================
// Keyboard accessibility (panel toggle + Esc to close path-pick)
// ============================================================
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    setPickMode(null);
    tooltip.classList.remove('visible');
    const tag = (e.target && e.target.tagName) || '';
    // Leave a text field alone. A focused menu still clears the graph highlight.
    if (tag === 'INPUT' || tag === 'TEXTAREA') return;
    setSelectedNode(null);
  }
});

// ============================================================
// Initial fit
// ============================================================
cy.one('layoutstop', () => {
  cy.fit(undefined, 40);
});

// ============================================================
// ===  v2 module: state, blast radius, focus, URL hash,
// ===            guided tour, mobile interactions
// ============================================================

// Central state object — single source of truth for URL hash encoding.
const state = {
  selectedNodeId: null,
  blastEnabled: true,        // on by default once a node is selected
  focusModeActive: false,    // off by default
  focusDepth: '1',           // click-focus hop radius: 1, 2, or all
  currentTour: null,         // { tourId, stepIndex }
};

// Look up the full node record from NODES by id (we passed reconstructed
// objects to renderNodePanel originally; the unified setSelectedNode does
// not need that).
const NODE_BY_ID = new Map(NODES.map(n => [n.id, n]));

// ----------------------------------------------------------------
// Inject style rules for the v2 cytoscape classes that didn't exist
// in v1. Done programmatically so we don't touch the cy() init block.
// ----------------------------------------------------------------
cy.style()
  .selector('.blast-direct')
    .style({ 'border-color': '#f0883e', 'border-width': 5, 'opacity': 1.0,
             'background-blacken': -0.15 })
  .selector('.blast-h2')
    .style({ 'border-color': '#f0883e', 'border-width': 3, 'opacity': 0.75 })
  .selector('.blast-h3')
    .style({ 'border-color': '#f0883e', 'border-width': 2, 'opacity': 0.55 })
  .selector('.blast-faded')
    .style({ 'opacity': 0.20, 'text-opacity': 0.2 })
  // v2.1: blast-edge thickens but retains the source→target domain gradient
  // (preserves the cross-domain visual signal — see spec note option (b)).
  .selector('edge.blast-edge')
    .style({ 'opacity': 0.95, 'width': 3.6, 'z-index': 50 })
  .selector('.focus-hidden')
    .style({ 'opacity': 0.05, 'text-opacity': 0 })
  .selector('.tour-highlight')
    .style({ 'border-color': '#D55E00', 'border-width': 5 })
  // Click focus. Later blast rules keep a 2-hop or 3-hop dependent visible
  // when Hops is 1. The Focus toggle's rule stays last so it can still hide
  // everything outside one hop.
  .selector('.nb-dim')
    .style({ 'opacity': 0.08, 'text-opacity': 0 })
  .selector('.nb-self')
    .style({ 'opacity': 1, 'text-opacity': 1, 'border-color': '#58a6ff',
             'border-width': 5, 'z-index': 12 })
  .selector('.nb-dep')
    .style({ 'opacity': 1, 'text-opacity': 1, 'border-color': '#3fb950',
             'border-width': 4 })
  .selector('.nb-user')
    .style({ 'opacity': 1, 'text-opacity': 1, 'border-color': '#f0883e',
             'border-width': 4 })
  .selector('edge.nb-edge')
    .style({ 'opacity': 1, 'width': 2.6, 'z-index': 8 })
  .selector('edge.nb-dim')
    .style({ 'opacity': 0.05 })
  .selector('.blast-direct')
    .style({ 'border-color': '#f0883e', 'border-width': 5, 'opacity': 1.0,
             'background-blacken': -0.15 })
  .selector('.blast-h2')
    .style({ 'border-color': '#f0883e', 'border-width': 3, 'opacity': 0.75 })
  .selector('.blast-h3')
    .style({ 'border-color': '#f0883e', 'border-width': 2, 'opacity': 0.55 })
  .selector('edge.blast-edge')
    .style({ 'opacity': 0.95, 'width': 3.6, 'z-index': 50 })
  .selector('.focus-hidden')
    .style({ 'opacity': 0.05, 'text-opacity': 0 })
  .update();

// ----------------------------------------------------------------
// Blast-radius computation (BFS over INCOMING edges — i.e. "who
// would be affected if this node changes")
// ----------------------------------------------------------------
function computeBlastRadius(nodeId, maxHops = 3) {
  const direct = new Set();
  const hop2 = new Set();
  const hop3 = new Set();
  const seen = new Set([nodeId]);
  let frontier = [nodeId];
  for (let hop = 1; hop <= maxHops; hop++) {
    const next = [];
    frontier.forEach(id => {
      const inEdges = incoming.get(id) || [];
      inEdges.forEach(e => {
        if (seen.has(e.source)) return;
        seen.add(e.source);
        next.push(e.source);
        if (hop === 1) direct.add(e.source);
        else if (hop === 2) hop2.add(e.source);
        else if (hop === 3) hop3.add(e.source);
      });
    });
    frontier = next;
    if (!frontier.length) break;
  }
  return { direct, hop2, hop3, all: seen };
}

function applyBlastHighlight() {
  // Clear previous
  cy.elements().removeClass('blast-direct blast-h2 blast-h3 blast-faded blast-edge');
  if (!state.selectedNodeId || !state.blastEnabled) return;
  const r = computeBlastRadius(state.selectedNodeId);
  // Fade the whole graph except: selected node, direct, hop2, hop3
  const keep = new Set([state.selectedNodeId, ...r.direct, ...r.hop2, ...r.hop3]);
  crateNodes().forEach(n => {
    if (!keep.has(n.id())) n.addClass('blast-faded');
  });
  r.direct.forEach(id => cy.getElementById(id).addClass('blast-direct'));
  r.hop2.forEach(id => cy.getElementById(id).addClass('blast-h2'));
  r.hop3.forEach(id => cy.getElementById(id).addClass('blast-h3'));
  // Edges connecting blast-radius nodes
  cy.edges().forEach(e => {
    if (keep.has(e.data('source')) && keep.has(e.data('target'))) {
      e.addClass('blast-edge');
    } else {
      e.addClass('blast-faded');
    }
  });
}

// ----------------------------------------------------------------
// Focus mode (1-hop neighborhood — both directions)
// ----------------------------------------------------------------
function applyFocusMode() {
  cy.elements().removeClass('focus-hidden');
  if (!state.focusModeActive || !state.selectedNodeId) return;
  const neighbors = new Set([state.selectedNodeId]);
  (outgoing.get(state.selectedNodeId) || []).forEach(e => neighbors.add(e.target));
  (incoming.get(state.selectedNodeId) || []).forEach(e => neighbors.add(e.source));
  crateNodes().forEach(n => {
    if (!neighbors.has(n.id())) n.addClass('focus-hidden');
  });
  cy.edges().forEach(e => {
    if (!neighbors.has(e.data('source')) || !neighbors.has(e.data('target'))) {
      e.addClass('focus-hidden');
    }
  });
}

function clearHighlights() {
  cy.elements().removeClass('blast-direct blast-h2 blast-h3 blast-faded blast-edge focus-hidden nb-dim nb-self nb-dep nb-user nb-edge');
}

// Click focus. Dependencies are crates reached by following depends-on edges
// (source → target) up to H hops. Dependents are crates reached by following
// those edges backward. "all" walks until the directed cone is exhausted.
function focusHopLimit() {
  const el = document.getElementById('focus-depth');
  const v = (el && el.value) || state.focusDepth || '1';
  if (v === 'all') return NODES.length + 1;
  return v === '2' ? 2 : 1;
}
function directionalHops(nodeId, maxHops, dir) {
  const found = new Set();
  const seen = new Set([nodeId]);
  let frontier = [nodeId];
  for (let hop = 1; hop <= maxHops && frontier.length; hop++) {
    const next = [];
    frontier.slice().sort(cmpStr).forEach(id => {
      const edges = dir === 'out' ? (outgoing.get(id) || []) : (incoming.get(id) || []);
      const nextIds = edges.map(e => dir === 'out' ? e.target : e.source).sort(cmpStr);
      nextIds.forEach(nid => {
        if (!nid || seen.has(nid)) return;
        seen.add(nid);
        found.add(nid);
        next.push(nid);
      });
    });
    frontier = next;
  }
  return found;
}
function applyNeighborhood() {
  cy.elements().removeClass('nb-dim nb-self nb-dep nb-user nb-edge');
  if (!state.selectedNodeId) return;
  const hops = focusHopLimit();
  const deps = directionalHops(state.selectedNodeId, hops, 'out');
  const users = directionalHops(state.selectedNodeId, hops, 'in');
  const keep = new Set([state.selectedNodeId, ...deps, ...users]);
  crateNodes().forEach(n => {
    const id = n.id();
    if (id === state.selectedNodeId) n.addClass('nb-self');
    else if (deps.has(id)) n.addClass('nb-dep');
    else if (users.has(id)) n.addClass('nb-user');
    else n.addClass('nb-dim');
    if (deps.has(id) && users.has(id) && id !== state.selectedNodeId) n.addClass('nb-user');
  });
  cy.edges().forEach(e => {
    if (keep.has(e.data('source')) && keep.has(e.data('target'))) e.addClass('nb-edge');
    else e.addClass('nb-dim');
  });
}

// ----------------------------------------------------------------
// Unified selection entry point
// ----------------------------------------------------------------
function setSelectedNode(id, opts = {}) {
  const skipHashUpdate = !!opts.skipHashUpdate;
  if (!id) {
    state.selectedNodeId = null;
    clearHighlights();
    cy.$(':selected').unselect();
    panelContent.innerHTML = '<div class="empty-panel">Click any node or edge to inspect.</div>';
    if (!skipHashUpdate) updateHash();
    return;
  }
  const node = NODE_BY_ID.get(id);
  if (!node) return;
  state.selectedNodeId = id;
  // Update Cytoscape selection visually
  cy.$(':selected').unselect();
  const cyNode = cy.getElementById(id);
  if (cyNode.length) cyNode.select();
  renderNodePanel(node);
  // Apply visual modes
  applyBlastHighlight();
  applyFocusMode();
  applyNeighborhood();
  if (!skipHashUpdate) updateHash();
}

function toggleBlast() {
  state.blastEnabled = !state.blastEnabled;
  const btn = document.getElementById('blast-toggle');
  btn.textContent = `Blast radius: ${state.blastEnabled ? 'on' : 'off'}`;
  btn.setAttribute('aria-pressed', state.blastEnabled ? 'true' : 'false');
  applyBlastHighlight();
  if (state.selectedNodeId) renderNodePanel(NODE_BY_ID.get(state.selectedNodeId));
  updateHash();
}
function toggleFocus() {
  state.focusModeActive = !state.focusModeActive;
  const btn = document.getElementById('focus-mode');
  btn.textContent = `Focus: ${state.focusModeActive ? 'on' : 'off'}`;
  btn.setAttribute('aria-pressed', state.focusModeActive ? 'true' : 'false');
  applyFocusMode();
  if (state.selectedNodeId) renderNodePanel(NODE_BY_ID.get(state.selectedNodeId));
  updateHash();
}
function centerOnNode(id) {
  const n = cy.getElementById(id);
  if (!n.length) return;
  cy.animate({ center: { eles: n }, zoom: 1.4 }, { duration: 350 });
}

// Wire toolbar toggles
document.getElementById('blast-toggle').addEventListener('click', toggleBlast);
document.getElementById('focus-mode').addEventListener('click', toggleFocus);
document.getElementById('focus-depth').addEventListener('change', () => {
  state.focusDepth = document.getElementById('focus-depth').value;
  applyNeighborhood();
  updateHash();
});

// Override v1's cy.on('tap', 'node') to route through setSelectedNode.
// We can't easily un-bind the v1 handler (it doesn't use a named function),
// so we add a second handler that runs AFTER it and synchronises state.
cy.on('tap', 'node', (evt) => {
  const t = evt.target;
  if (t.isParent()) return;
  if (pickMode === 'A' || pickMode === 'B') return; // path-pick fall-through
  setSelectedNode(t.id(), { skipHashUpdate: false });
});
// Background tap clears selection (and clears blast/focus side-effects)
cy.on('tap', (evt) => {
  if (evt.target === cy) {
    if (state.selectedNodeId) setSelectedNode(null);
  }
});

// ----------------------------------------------------------------
// Improvement 1: URL-hash state persistence
// ----------------------------------------------------------------
function encodeState() {
  const parts = [];
  if (state.selectedNodeId) parts.push('selected=' + encodeURIComponent(state.selectedNodeId));
  const story = storySelect.value;
  if (story && story !== 'free') parts.push('story=' + encodeURIComponent(story));
  // v2.1: domains encoded as the include-set, only when filter is active
  // (non-empty proper subset). Empty and full sets are both "no filter."
  if (isDomainFilterActive()) {
    parts.push('domains=' + encodeURIComponent([...enabledDomains].join(',')));
  }
  if (dormancyFilter.value && dormancyFilter.value !== 'all') {
    parts.push('dormancy=' + encodeURIComponent(dormancyFilter.value));
  }
  if (searchInput.value) parts.push('q=' + encodeURIComponent(searchInput.value));
  const lay = document.getElementById('layout-select').value;
  if (lay && lay !== 'layered') parts.push('layout=' + encodeURIComponent(lay));
  const hops = document.getElementById('focus-depth').value;
  if (hops && hops !== '1') parts.push('hops=' + encodeURIComponent(hops));
  if (sidePanel.classList.contains('collapsed')) parts.push('panel=closed');
  if (state.focusModeActive) parts.push('focus=1');
  if (!state.blastEnabled) parts.push('blast=0');
  if (pathA && pathB) parts.push('path=' + encodeURIComponent(pathA + '>>' + pathB));
  if (state.currentTour) {
    parts.push('tour=' + encodeURIComponent(state.currentTour.tourId + ':' + state.currentTour.stepIndex));
  }
  return parts.length ? '#' + parts.join('&') : '';
}

let hashUpdateScheduled = false;
function updateHash() {
  if (hashUpdateScheduled) return;
  hashUpdateScheduled = true;
  // Debounce: coalesce rapid state changes into one history entry.
  requestAnimationFrame(() => {
    hashUpdateScheduled = false;
    const newHash = encodeState();
    if (newHash !== window.location.hash) {
      try {
        history.replaceState(null, '', window.location.pathname +
                             window.location.search + newHash);
      } catch (_) {
        window.location.hash = newHash;
      }
    }
  });
}

function decodeHash() {
  const raw = (window.location.hash || '').replace(/^#/, '');
  if (!raw) return {};
  const out = {};
  raw.split('&').forEach(pair => {
    const [k, v] = pair.split('=');
    if (!k) return;
    out[decodeURIComponent(k)] = v == null ? true : decodeURIComponent(v);
  });
  return out;
}

function restoreFromHash() {
  const s = decodeHash();
  if (!Object.keys(s).length) return;
  // Domains — v2.1: payload is the include set; default = empty (no filter)
  if (s.domains) {
    const wanted = new Set(s.domains.split(','));
    enabledDomains.clear();
    DOMAINS.forEach((d, i) => {
      const chip = domainFilters.querySelectorAll('.domain-chip')[i];
      if (wanted.has(d.id)) {
        enabledDomains.add(d.id);
        chip.classList.add('on');
        chip.setAttribute('aria-pressed', 'true');
      } else {
        chip.classList.remove('on');
        chip.setAttribute('aria-pressed', 'false');
      }
    });
  }
  // Dormancy
  if (s.dormancy) dormancyFilter.value = s.dormancy;
  // Search
  if (s.q) searchInput.value = s.q;
  // Layout
  if (s.layout) {
    const sel = document.getElementById('layout-select');
    if ([...sel.options].some(o => o.value === s.layout)) {
      sel.value = s.layout;
      // runInitialLayout already applied this choice. Running it again would
      // reshuffle fcose. Re-explode is the control that does that on purpose.
      if (currentLayoutName !== s.layout) runLayout(s.layout);
    }
  }
  if (s.hops === '1' || s.hops === '2' || s.hops === 'all') {
    const hopSel = document.getElementById('focus-depth');
    hopSel.value = s.hops;
    state.focusDepth = s.hops;
  }
  // Apply filters now that domain/dormancy/q are set
  applyFilters();
  // Story (override layout dimming)
  if (s.story) {
    storySelect.value = s.story;
    applyStory(s.story);
  }
  // Focus + blast
  if (s.focus === '1' || s.focus === true) {
    state.focusModeActive = true;
    const btn = document.getElementById('focus-mode');
    btn.textContent = 'Focus: on';
    btn.setAttribute('aria-pressed', 'true');
  }
  if (s.blast === '0') {
    state.blastEnabled = false;
    const btn = document.getElementById('blast-toggle');
    btn.textContent = 'Blast radius: off';
    btn.setAttribute('aria-pressed', 'false');
  }
  // Panel state
  if (s.panel === 'closed') {
    sidePanel.classList.add('collapsed');
    document.getElementById('panel-toggle').textContent = '‹ Open';
  }
  // Selected node (must come after applyStory so it overrides dimming)
  if (s.selected) {
    setSelectedNode(s.selected, { skipHashUpdate: true });
  }
  // Path
  if (s.path && s.path.includes('>>')) {
    const [a, b] = s.path.split('>>');
    pathA = a; pathB = b;
    document.getElementById('path-a').textContent = a;
    document.getElementById('path-b').textContent = b;
    updatePathBtn();
    document.getElementById('show-path').click();
  }
  // Tour (must be last — overrides other view state with tour's preset)
  if (s.tour && s.tour.includes(':')) {
    const [tourId, stepStr] = s.tour.split(':');
    const stepIdx = parseInt(stepStr, 10);
    if (TOURS.find(t => t.id === tourId) && !isNaN(stepIdx)) {
      startTour(tourId, stepIdx);
    }
  }
}

// Hook the existing filter/story/layout/panel handlers to update the hash
// after their state changes. We attach these as additional listeners so we
// don't have to rewrite the v1 handlers.
[searchInput, dormancyFilter, document.getElementById('layout-select'),
 document.getElementById('story-select')].forEach(el => {
  el.addEventListener('change', updateHash);
  if (el.tagName === 'INPUT') el.addEventListener('input', updateHash);
});
// Domain chip clicks: the v1 listener mutates enabledDomains then calls
// applyFilters(); we just listen on the parent and call updateHash() after.
domainFilters.addEventListener('click', () => requestAnimationFrame(updateHash));
document.getElementById('panel-toggle').addEventListener('click',
  () => requestAnimationFrame(updateHash));

// Share button
function showToast(msg, durationMs = 2200) {
  const t = document.getElementById('toast');
  t.innerHTML = msg;
  t.classList.add('visible');
  clearTimeout(showToast._timer);
  showToast._timer = setTimeout(() => t.classList.remove('visible'), durationMs);
}
document.getElementById('share-view').addEventListener('click', () => {
  updateHash();
  setTimeout(() => {
    const url = window.location.href;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(url).then(
        () => showToast('<strong>Copied URL ✓</strong> — paste it to share this exact view'),
        () => showToast(`<strong>Copy failed.</strong> URL: <code>${escapeHtml(url)}</code>`)
      );
    } else {
      showToast(`<strong>URL:</strong> <code>${escapeHtml(url)}</code>`);
    }
  }, 50); // give updateHash's rAF a tick
});

// ----------------------------------------------------------------
// Improvement 3: Guided tour
// ----------------------------------------------------------------
const TOURS = [
  {
    id: 'onboarding',
    title: 'First-time onboarding',
    duration: '~10 minutes',
    steps: [
      {
        title: 'The eight domains',
        preset: 'free',
        customViz: (cy) => { /* free exploration, no filtering */ },
        narration: `The default layout is <strong>dependency tiers</strong>. Crates with no workspace dependency, including <code>astraweave-math</code>, sit on the top row. Each row down depends on rows above it, and <code>aw_editor</code> and the examples bundle sit at the bottom. Arrowheads point at the dependency, so they run mostly upward. Switch Layout to <strong>Domain clusters (fcose)</strong> to see the eight domain compound boxes: Core, AI, Rendering, Physics &amp; World, Gameplay, Networking, Tools, and the collapsed Examples bundle. Colour is the Wong colourblind-safe palette. Node size encodes LoC band; dashed borders mark dormant crates; <code>⚠</code> overlays mark silent-failure loci; <code>📄</code> overlays mark traced subsystems.`,
      },
      {
        title: 'The foundation — Core domain',
        preset: null,
        customViz: () => filterToDomains(['core']),
        narration: `<code>astraweave-ecs</code>, <code>astraweave-math</code>, <code>astraweave-core</code>, and <code>astraweave-sdk</code> are the foundation layer that nearly everything else depends on. Two structural facts to internalise now: (1) the workspace carries <strong>two <code>World</code> types</strong> — the modern archetype substrate in <code>astraweave-ecs</code> and a legacy HashMap-per-component <code>World</code> still alive in <code>astraweave-core</code>, bridged by <code>build_app</code>. (2) <code>ParallelSchedule</code> was removed 2026-04-18; the scheduler is now deterministic single-threaded. See <code>ecs_math_core_sdk_foundation.md</code> §1, §6, §7.`,
      },
      {
        title: 'The AI-first commitment',
        preset: null,
        customViz: () => filterToDomains(['ai']),
        narration: `The AI pipeline is the engine's first-class citizen — validated at <strong>12,700+ agents @ 60 FPS</strong>. The canonical loop is <strong>Perception → Reasoning → Planning → Action</strong>: <code>build_ai_snapshots</code> → <code>WorldSnapshot</code> → <code>AIArbiter</code> → <code>Orchestrator.plan()</code> → <code>PlanIntent</code> → <code>tool_sandbox</code> → engine-side <code>validate_and_execute</code>. <code>ai_pipeline.md</code> documents eight subsystem traces under this domain — and most of those subsystems are <strong>dormant</strong>: Memory (~11K LoC), Coordination (~5.3K), RAG composite (~12.3K), Advanced GOAP (~16.7K), LLM Production Hardening (~15K). Two GOAP implementations coexist; the runtime LLM default is <code>phi3:medium</code> despite Qwen3 doc-comments.`,
      },
      {
        title: 'The rendering surface',
        preset: 'editor_engine_render',
        narration: `<strong>Fix-27</strong> (2026-04-04) eliminated the editor's parallel FastPreview rendering pipeline — ~4,000 LoC deleted, <code>astraweave-render</code> is now a REQUIRED non-optional dep of <code>aw_editor</code>. The editor-engine boundary is the most critical integration seam in the workspace; the load-bearing surface is <code>Renderer::draw_into(view, depth_view, encoder)</code> (Real-Fix.A 2026-05-07). Real-Fix.B/C unified terrain mesh and texture flows. Real-Fix.D is pending. See <code>render_pipeline_material_system_shader_infrastructure.md</code> §6, §7 and ARCHITECTURE_MAP §6.5.`,
      },
      {
        title: 'The dual-World bridge',
        preset: 'dual_world_bridge',
        narration: `Take a closer look at the transitional coexistence. <code>astraweave_ecs::World</code> (archetype substrate, generational <code>Entity{id, generation}</code>) lives alongside <code>astraweave_core::World</code> (HashMap-per-component, bare <code>u32</code> Entity). They are bridged inside an <code>App</code> by <code>astraweave_core::ecs_adapter::build_app</code>. Downstream consumers see either or both depending on which Resource they request. There is no retirement campaign for the legacy World; this is open question <strong>Q1</strong> in §14 of the architecture map.`,
      },
      {
        title: "What's not running — dormant surface",
        preset: 'dormant_surface',
        narration: `Around <strong>200K LoC across the workspace is dormant</strong>: passes its own tests, has zero production consumers. This is the signature of breadth-first AI-augmented development, <em>not</em> a flaw — many of these subsystems are forward-design infrastructure waiting for production wiring. Largest single reservoir is <code>astraweave-fluids</code> (~84.5K, only <code>examples/fluids_demo</code> uses it). See §5 of the architecture map for the six-category inventory. Before adding any new subsystem, check this view first — there's a good chance the surface already exists.`,
      },
      {
        title: 'Silent failures — where to look when something silently no-ops',
        preset: 'silent_failures',
        narration: `Twelve load-bearing silent-failure shapes survive across the workspace per §4.3: <code>let _ =</code> / <code>.ok()</code> on user-facing fallible operations, TODO-only function bodies, no-op stubs that look like real functions. If a feature "silently no-ops" the bug is overwhelmingly likely to be one of these. Click any highlighted node to see the file:line citations. <code>auto_save_system</code> drops a failed serialize and the first save error. <code>aw-net-proto</code> verifies a 32-byte HMAC-SHA256 tag, and the server kicks by default. The 16-byte <code>sign16</code> path is gone. <code>safe_llm_invoke</code> has zero workspace callers.`,
      },
      {
        title: 'Where to go next',
        preset: 'free',
        customViz: () => filterToDomains(DOMAINS.map(d => d.id)),
        narration: `You now have the workspace's load-bearing skeleton. Suggested next steps: <strong>(1)</strong> read <code>ARCHITECTURE_MAP.md</code> end-to-end, <strong>(2)</strong> pick a traced subsystem matching your interest and read its trace (the trace files under <code>docs/architecture/</code> are forensic references — file map, conflict map, decision log, invariants, open questions), <strong>(3)</strong> use the <strong>Blast radius</strong> toggle on any node before modifying it — the panel shows direct and transitive dependents within 3 hops. The <strong>Share</strong> button copies a URL that reproduces your current view, useful for debugging architectural questions with a colleague.`,
      },
    ],
  },
];

function filterToDomains(domainIds) {
  // v2.1: Helper for tour custom-viz. Sets the include-set to the given list
  // and lights up the matching chips. When domainIds === all 8 domains, the
  // applyFilters() call treats it as "no filter" (full set special case).
  enabledDomains.clear();
  domainIds.forEach(id => enabledDomains.add(id));
  DOMAINS.forEach((d, i) => {
    const chip = domainFilters.querySelectorAll('.domain-chip')[i];
    if (enabledDomains.has(d.id)) {
      chip.classList.add('on');
      chip.setAttribute('aria-pressed', 'true');
    } else {
      chip.classList.remove('on');
      chip.setAttribute('aria-pressed', 'false');
    }
  });
  storySelect.value = 'free';
  applyStory('free');
  applyFilters();
  // Fit to visible
  setTimeout(() => {
    const vis = crateNodes().filter(n => !n.hasClass('hidden'));
    if (vis.length) cy.fit(vis, 80);
  }, 100);
}

// Populate tour selector
const tourSelect = document.getElementById('tour-select');
TOURS.forEach(t => {
  const opt = document.createElement('option');
  opt.value = t.id;
  opt.textContent = `${t.title} (${t.duration})`;
  tourSelect.appendChild(opt);
});

function startTour(tourId, stepIdx = 0) {
  const tour = TOURS.find(t => t.id === tourId);
  if (!tour) return;
  state.currentTour = { tourId, stepIndex: Math.max(0, Math.min(stepIdx, tour.steps.length - 1)) };
  tourSelect.value = tourId;
  setChipsEnabled(false);  // v2.1: lock filter chips while a tour is in progress
  renderTourStep();
  updateHash();
}

function renderTourStep() {
  if (!state.currentTour) return;
  const tour = TOURS.find(t => t.id === state.currentTour.tourId);
  const step = tour.steps[state.currentTour.stepIndex];
  if (!step) return;
  // Show overlay, hide free banner
  document.getElementById('tour-overlay').classList.add('active');
  document.getElementById('banner').classList.remove('active');
  document.getElementById('tour-title').textContent = step.title;
  document.getElementById('tour-progress').textContent =
    `Step ${state.currentTour.stepIndex + 1} of ${tour.steps.length}`;
  document.getElementById('tour-narration').innerHTML = step.narration;
  document.getElementById('tour-prev').disabled = state.currentTour.stepIndex === 0;
  document.getElementById('tour-next').disabled =
    state.currentTour.stepIndex >= tour.steps.length - 1;
  // Apply step's visualization. Steps that don't set a domain filter must
  // clear one left behind by an earlier step, including after Prev.
  clearHighlights();
  clearDomainFilter();
  if (step.preset) {
    storySelect.value = step.preset;
    applyStory(step.preset);
  } else {
    storySelect.value = 'free';
    applyStory('free');
  }
  if (step.customViz) step.customViz(cy);
  applyFilters();
}
function tourNext() {
  if (!state.currentTour) return;
  const tour = TOURS.find(t => t.id === state.currentTour.tourId);
  if (state.currentTour.stepIndex < tour.steps.length - 1) {
    state.currentTour.stepIndex++;
    renderTourStep();
    updateHash();
  }
}
function tourPrev() {
  if (!state.currentTour) return;
  if (state.currentTour.stepIndex > 0) {
    state.currentTour.stepIndex--;
    renderTourStep();
    updateHash();
  }
}
function clearDomainFilter() {
  enabledDomains.clear();
  document.querySelectorAll('.domain-chip').forEach(chip => {
    chip.classList.remove('on');
    chip.setAttribute('aria-pressed', 'false');
  });
}
function tourExit() {
  state.currentTour = null;
  document.getElementById('tour-overlay').classList.remove('active');
  tourSelect.value = '';
  clearDomainFilter();
  storySelect.value = 'free';
  applyStory('free');
  setChipsEnabled(true);   // v2.1: restore filter-chip interactivity
  applyFilters();
  cy.fit(undefined, 40);
  updateHash();
}

tourSelect.addEventListener('change', (e) => {
  if (e.target.value) startTour(e.target.value);
});
document.getElementById('tour-next').addEventListener('click', tourNext);
document.getElementById('tour-prev').addEventListener('click', tourPrev);
document.getElementById('tour-exit').addEventListener('click', tourExit);

// Tour keyboard shortcuts
document.addEventListener('keydown', (e) => {
  if (!state.currentTour) return;
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
  if (e.key === 'ArrowRight') tourNext();
  else if (e.key === 'ArrowLeft') tourPrev();
});

// ----------------------------------------------------------------
// Improvement 6: Mobile interactions
// ----------------------------------------------------------------
// 6a. Hamburger toggles toolbar on mobile
document.getElementById('mobile-toolbar-toggle').addEventListener('click', (e) => {
  const tb = document.getElementById('toolbar');
  const open = tb.classList.toggle('mobile-open');
  e.target.setAttribute('aria-expanded', open ? 'true' : 'false');
});

// 6b. Bottom-sheet drag: clicking the handle expands/collapses the panel.
const sheetHandle = document.getElementById('mobile-sheet-handle');
if (sheetHandle) {
  sheetHandle.addEventListener('click', () => {
    sidePanel.classList.toggle('mobile-expanded');
  });
}

// 6c. On mobile, selecting a node should expand the sheet so the user can
//     actually read the panel.
function expandPanelOnMobile() {
  if (window.innerWidth <= 768) {
    sidePanel.classList.add('mobile-expanded');
  }
}
const origSetSelected = setSelectedNode;
// (already wired; just expand sheet when a node is selected)
cy.on('tap', 'node', (evt) => {
  if (!evt.target.isParent() && pickMode == null) expandPanelOnMobile();
});

// ----------------------------------------------------------------
// Reset view also clears v2 state
// ----------------------------------------------------------------
const v1ResetBtn = document.getElementById('reset-view');
v1ResetBtn.addEventListener('click', () => {
  state.selectedNodeId = null;
  state.focusModeActive = false;
  state.blastEnabled = true;
  state.focusDepth = '1';
  const hopSel = document.getElementById('focus-depth');
  if (hopSel) hopSel.value = '1';
  // Reset toggle button labels
  const fb = document.getElementById('focus-mode');
  fb.textContent = 'Focus: off';
  fb.setAttribute('aria-pressed', 'false');
  const bb = document.getElementById('blast-toggle');
  bb.textContent = 'Blast radius: on';
  bb.setAttribute('aria-pressed', 'true');
  clearHighlights();
  if (state.currentTour) tourExit();
  panelContent.innerHTML = '<div class="empty-panel">Click any node or edge to inspect.</div>';
  // Defer hash clear so v1 handler runs first
  requestAnimationFrame(() => updateHash());
});

// ----------------------------------------------------------------
// Restore state from URL hash on load (after fcose layout settles)
// ----------------------------------------------------------------
function restoreOnLoad() {
  try { restoreFromHash(); } catch (e) { console.warn('hash restore failed:', e); }
}
// Wait for initial layout to settle before restoring (so cy.fit calls work)
let restoreAttempts = 0;
function tryRestore() {
  // `:child` is empty once the default layout detaches domain boxes.
  if (!bootLayoutStarted || !crateNodes().length) {
    if (restoreAttempts++ > 40) { restoreOnLoad(); return; }
    setTimeout(tryRestore, 100);
    return;
  }
  restoreOnLoad();
}
applyFilters();
waitForContainerSize(document.getElementById('cy'), runInitialLayout);
setTimeout(tryRestore, 400);

// Listen for browser back/forward navigation (hash changes)
window.addEventListener('hashchange', restoreOnLoad);
