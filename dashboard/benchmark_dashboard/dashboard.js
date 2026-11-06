// AstraWeave Benchmark Dashboard v7.0 - Data Visualization & Analysis
// Reads history.jsonl and renders interactive d3.js charts
// Features: Baseline vs User hardware comparison, Industry Standard references,
//           Statistical analysis (percentiles, CI), CSV/JSON export, shareable URLs

// ─── DATA SOURCES ────────────────────────────────────────────────────────────
const DATA_SOURCES = [
    'benchmark-data/history.jsonl',              // Primary: relative path (works on GH Pages and local server)
    'static-data/history.jsonl',                 // Fallback: shipped static snapshot
    './benchmark-data/history.jsonl',            // Explicit relative fallback
    './static-data/history.jsonl',               // Explicit relative fallback (static)
    'https://raw.githubusercontent.com/lazyxeon/AstraWeave/main/tools/benchmark-dashboard/benchmark-data/history.jsonl',
    'https://raw.githubusercontent.com/lazyxeon/AstraWeave/main/tools/benchmark-dashboard/static-data/history.jsonl',
    '../../target/benchmark-data/history.jsonl', // Local dev (from criterion benchmarks)
];

// ─── INLINE FALLBACK DATA ─────────────────────────────────────────────────────
// Embedded static snapshot (12 entries) so the dashboard always renders something
// even when no server hosts the JSONL files (e.g. first-time GH Pages deploy).
const INLINE_FALLBACK_JSONL = [
  '{"group":"ecs_benchmarks","display_name":"ECS World Creation","unit":"ns","git_sha":"8b3c1a0d","crate":"astraweave-core","git_branch":"main","stddev":0.31,"timestamp":"2025-11-10T15:00:00Z","name":"world_creation","benchmark_name":"astraweave-core::ecs_benchmarks/world_creation","git_dirty":false,"value":25.83,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"ecs_benchmarks","display_name":"ECS World Creation","unit":"ns","git_sha":"4f71cbb1","crate":"astraweave-core","git_branch":"main","stddev":0.27,"timestamp":"2025-11-12T15:00:00Z","name":"world_creation","benchmark_name":"astraweave-core::ecs_benchmarks/world_creation","git_dirty":false,"value":25.64,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"ecs_benchmarks","display_name":"ECS World Creation","unit":"ns","git_sha":"2a9bd055","crate":"astraweave-core","git_branch":"main","stddev":0.29,"timestamp":"2025-11-14T15:00:00Z","name":"world_creation","benchmark_name":"astraweave-core::ecs_benchmarks/world_creation","git_dirty":false,"value":25.59,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"ai_core_loop","display_name":"AI Orchestrator Update","unit":"ns","git_sha":"8b3c1a0d","crate":"astraweave-ai","git_branch":"main","stddev":0.8,"timestamp":"2025-11-10T15:05:00Z","name":"update_orchestrator","benchmark_name":"astraweave-ai::ai_core_loop/update_orchestrator","git_dirty":false,"value":184.22,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"ai_core_loop","display_name":"AI Orchestrator Update","unit":"ns","git_sha":"4f71cbb1","crate":"astraweave-ai","git_branch":"main","stddev":0.74,"timestamp":"2025-11-12T15:05:00Z","name":"update_orchestrator","benchmark_name":"astraweave-ai::ai_core_loop/update_orchestrator","git_dirty":false,"value":182.11,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"ai_core_loop","display_name":"AI Orchestrator Update","unit":"ns","git_sha":"2a9bd055","crate":"astraweave-ai","git_branch":"main","stddev":0.69,"timestamp":"2025-11-14T15:05:00Z","name":"update_orchestrator","benchmark_name":"astraweave-ai::ai_core_loop/update_orchestrator","git_dirty":false,"value":181.67,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"character_controller","display_name":"Physics Character Controller","unit":"us","git_sha":"8b3c1a0d","crate":"astraweave-physics","git_branch":"main","stddev":0.05,"timestamp":"2025-11-10T15:10:00Z","name":"full_tick","benchmark_name":"astraweave-physics::character_controller/full_tick","git_dirty":false,"value":6.54,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"character_controller","display_name":"Physics Character Controller","unit":"us","git_sha":"4f71cbb1","crate":"astraweave-physics","git_branch":"main","stddev":0.04,"timestamp":"2025-11-12T15:10:00Z","name":"full_tick","benchmark_name":"astraweave-physics::character_controller/full_tick","git_dirty":false,"value":6.52,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"character_controller","display_name":"Physics Character Controller","unit":"us","git_sha":"2a9bd055","crate":"astraweave-physics","git_branch":"main","stddev":0.05,"timestamp":"2025-11-14T15:10:00Z","name":"full_tick","benchmark_name":"astraweave-physics::character_controller/full_tick","git_dirty":false,"value":6.49,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"mesh_optimization","display_name":"Vertex Compression","unit":"ns","git_sha":"8b3c1a0d","crate":"astraweave-render","git_branch":"main","stddev":0.11,"timestamp":"2025-11-10T15:15:00Z","name":"vertex_compression","benchmark_name":"astraweave-render::mesh_optimization/vertex_compression","git_dirty":false,"value":21.18,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"mesh_optimization","display_name":"Vertex Compression","unit":"ns","git_sha":"4f71cbb1","crate":"astraweave-render","git_branch":"main","stddev":0.09,"timestamp":"2025-11-12T15:15:00Z","name":"vertex_compression","benchmark_name":"astraweave-render::mesh_optimization/vertex_compression","git_dirty":false,"value":20.98,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
  '{"group":"mesh_optimization","display_name":"Vertex Compression","unit":"ns","git_sha":"2a9bd055","crate":"astraweave-render","git_branch":"main","stddev":0.1,"timestamp":"2025-11-14T15:15:00Z","name":"vertex_compression","benchmark_name":"astraweave-render::mesh_optimization/vertex_compression","git_dirty":false,"value":20.88,"hardware_id":"aed36a43","hardware_label":"HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)"}',
];

// ─── HP PAVILION BASELINE HARDWARE ID ────────────────────────────────────────
const BASELINE_HARDWARE_ID = 'aed36a43';
const BASELINE_HARDWARE_LABEL = 'HP Pavilion Gaming Laptop 16-a0xxx (Intel i5-10300H, 32GB)';

// ─── INDUSTRY STANDARD REFERENCE VALUES ──────────────────────────────────────
// Representative benchmarks from comparable engines/frameworks.
// Sources: Bevy 0.14, Flecs, Unity DOTS, Godot, Rapier3D, glam, nalgebra published benchmarks.
// Values in nanoseconds unless noted.
const INDUSTRY_STANDARDS = {
    ecs: {
        label: 'ECS Industry Avg',
        benchmarks: {
            'world_creation':       { bevy: 35, unity_dots: 120, godot: 250, flecs: 28 },
            'entity_spawn':         { bevy: 500, unity_dots: 800, godot: 1200, flecs: 380 },
            'entity_spawn_batch':   { bevy: 180, unity_dots: 400, godot: 900, flecs: 150 },
            'query_iteration':      { bevy: 2.5, unity_dots: 4.0, godot: 15, flecs: 2.0 },
            'archetype_move':       { bevy: 800, unity_dots: 600, godot: 2000, flecs: 500 },
            'event_dispatch':       { bevy: 45, unity_dots: 80, godot: 200, flecs: 35 },
            'system_execution':     { bevy: 150, unity_dots: 250, godot: 500, flecs: 120 },
        }
    },
    ai: {
        label: 'AI Industry Avg',
        benchmarks: {
            'behavior_tree':                { typical_engine: 200, utility_ai_sdk: 350 },
            'behavior_tree_10_nodes':       { typical_engine: 250, utility_ai_sdk: 400 },
            'behavior_tree_20_nodes':       { typical_engine: 600, utility_ai_sdk: 900 },
            'behavior_tree_simple_3_nodes': { typical_engine: 80, utility_ai_sdk: 120 },
            'behavior_tree_condition':      { typical_engine: 100, utility_ai_sdk: 150 },
            'behavior_tree_decorator':      { typical_engine: 60, utility_ai_sdk: 90 },
            'behavior_tree_sequence':       { typical_engine: 180, utility_ai_sdk: 280 },
            'goap_planning_simple':         { typical_engine: 8000, strips_planner: 12000 },
            'goap_planning_10_actions':     { typical_engine: 25000, strips_planner: 45000 },
            'goap_planning_20_actions':     { typical_engine: 80000, strips_planner: 150000 },
            'goap_goal_evaluation':         { typical_engine: 3000, strips_planner: 5000 },
            'goap_action_preconditions':    { typical_engine: 1500, strips_planner: 3000 },
            'goap_caching':                 { typical_engine: 5000, strips_planner: 8000 },
        }
    },
    physics: {
        label: 'Physics Industry Avg',
        benchmarks: {
            'character_controller':    { rapier: 5000, physx: 3500, bullet: 8000 },
            'raycast':                 { rapier: 200, physx: 150, bullet: 350 },
            'collision_detection':     { rapier: 1500, physx: 1200, bullet: 3000 },
            'spatial_hash':            { rapier: 800, custom: 400 },
        }
    },
    math: {
        label: 'Math Industry Avg',
        benchmarks: {
            'vec3_dot':        { glam: 1.8, nalgebra: 3.5, directxmath: 1.5 },
            'vec3_cross':      { glam: 2.2, nalgebra: 4.0, directxmath: 1.8 },
            'vec3_normalize':  { glam: 3.5, nalgebra: 6.0, directxmath: 2.8 },
            'mat4_mul':        { glam: 8.0, nalgebra: 12.0, directxmath: 6.5 },
            'quat_multiply':   { glam: 3.0, nalgebra: 5.5, directxmath: 2.5 },
            'quat_slerp':      { glam: 12.0, nalgebra: 18.0, directxmath: 9.0 },
            'transform':       { glam: 5.0, nalgebra: 8.0, directxmath: 4.0 },
        }
    },
    rendering: {
        label: 'Rendering Industry Avg',
        benchmarks: {
            'culling_performance':  { typical_engine: 50000, unreal: 35000, godot: 80000 },
            'vertex_compression':   { typical_engine: 25, meshopt: 18 },
            'lod_generation':       { typical_engine: 500000, meshoptimizer: 350000 },
            'mesh_simplification':  { meshoptimizer: 400000, typical_engine: 600000 },
            'instance_batching':    { typical_engine: 5000, unreal: 3000 },
            'bloom':                { typical_engine: 50000000, unreal: 40000000 },
            'shadow':               { typical_engine: 100000, unreal: 80000 },
            'frustum':              { typical_engine: 15000, unreal: 10000 },
            'depth_sort':           { typical_engine: 2000, custom: 1500 },
            'material_compile':     { typical_engine: 500000, unreal: 300000 },
        }
    },
    terrain: {
        label: 'Terrain Industry Avg',
        benchmarks: {
            'heightmap_generation':     { typical_engine: 500000, noise_rs: 300000 },
            'world_chunk_generation':   { minecraft_like: 30000000, typical_engine: 50000000 },
            'voxel_meshing':            { typical_engine: 200000, openvdb: 150000 },
            'climate_sampling':         { typical_engine: 50000, custom: 30000 },
        }
    },
    animation: {
        label: 'Animation Industry Avg',
        benchmarks: {
            'animation_blending':       { bevy: 900, unity: 600, godot: 1500 },
            'crossfade':                { bevy: 900, unity: 600, godot: 1500 },
            'animation_sample':         { bevy: 1200, unity: 800, godot: 2000 },
            'animation_hierarchy':      { bevy: 2000, unity: 1500, godot: 3500 },
            'forward_kinematics':       { bevy: 2000, unity: 1500, godot: 3500 },
            'animation_full_frame':     { bevy: 5000, unity: 3000, godot: 8000 },
            'joint_palette':            { bevy: 1500, unity: 1000, godot: 2500 },
            'keyframe_search':          { bevy: 30, unity: 25, godot: 60 },
            'animation_transform':      { bevy: 50, unity: 35, godot: 80 },
            'animation_matrix':         { bevy: 15, unity: 10, godot: 25 },
        }
    },
    ui: {
        label: 'UI Industry Avg',
        benchmarks: {
            'widget':       { egui: 2000, imgui: 1500, iced: 3000 },
            'gizmo':        { typical_engine: 5000, blender: 3000 },
        }
    },
    persistence: {
        label: 'Persistence Industry Avg',
        benchmarks: {
            'save':             { serde_json: 50000, bincode: 20000, rkyv: 10000 },
            'serialization':    { serde_json: 50000, bincode: 20000, rkyv: 10000 },
        }
    }
};

// ─── COLOR SCHEME ────────────────────────────────────────────────────────────
const COLOR_SCHEME = {
    ecs: '#4facfe',
    ai: '#00f2fe',
    physics: '#43e97b',
    terrain: '#fa709a',
    input: '#f093fb',
    rendering: '#feca57',
    math: '#ff6b6b',
    networking: '#48dbfb',
    persistence: '#ff9ff3',
    audio: '#54a0ff',
    ui: '#5f27cd',
    tools: '#00d2d3',
    animation: '#e17055',
    default: '#a0a0a0'
};

const INDUSTRY_LINE_COLOR = '#ff9500';

// ─── PERFORMANCE ─────────────────────────────────────────────────────────────
const PERF = { loadStart: performance.now(), renderCount: 0 };

// ─── STATISTICAL HELPERS ─────────────────────────────────────────────────────

function percentile(arr, p) {
    if (arr.length === 0) return 0;
    const sorted = [...arr].sort((a, b) => a - b);
    const idx = (p / 100) * (sorted.length - 1);
    const lo = Math.floor(idx), hi = Math.ceil(idx);
    return lo === hi ? sorted[lo] : sorted[lo] + (sorted[hi] - sorted[lo]) * (idx - lo);
}

function mean(arr) { return arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : 0; }

function stddev(arr) {
    if (arr.length < 2) return 0;
    const m = mean(arr);
    return Math.sqrt(arr.reduce((s, v) => s + (v - m) ** 2, 0) / (arr.length - 1));
}

function confidenceInterval95(arr) {
    if (arr.length < 2) return { lo: mean(arr), hi: mean(arr), margin: 0 };
    const m = mean(arr), s = stddev(arr);
    const margin = 1.96 * s / Math.sqrt(arr.length);
    return { lo: m - margin, hi: m + margin, margin };
}

function coefficientOfVariation(arr) {
    const m = mean(arr);
    return m !== 0 ? (stddev(arr) / m) * 100 : 0;
}

// ─── TOAST NOTIFICATION ──────────────────────────────────────────────────────

function showToast(msg, duration = 2500) {
    const el = document.getElementById('toast');
    if (!el) return;
    el.textContent = msg;
    el.classList.add('show');
    setTimeout(() => el.classList.remove('show'), duration);
}

// ─── DEBOUNCE ────────────────────────────────────────────────────────────────

function debounce(fn, ms) {
    let timer;
    return (...args) => { clearTimeout(timer); timer = setTimeout(() => fn(...args), ms); };
}

const debouncedRender = debounce(() => renderDashboard(), 120);

// ─── STATE ───────────────────────────────────────────────────────────────────
let benchmarkData = [];
let filteredData = [];
let hardwareProfiles = new Map();
let currentFilters = {
    system: 'all',
    timeRange: 'all',
    benchmark: null,
    hardware: 'all',
    showIndustry: true
};

// ─── DATA LOADING ────────────────────────────────────────────────────────────

async function tryLoadFromSource(source) {
    const response = await fetch(source);
    if (!response.ok) return null;
    const text = await response.text();
    const lines = text.split('\n')
        .map(l => l.trim())
        .filter(l => l.length > 0 && !l.startsWith('#') && l.startsWith('{'));
    if (lines.length === 0) return null;
    return lines;
}

async function loadBenchmarkData() {
    console.log('=== AstraWeave Benchmark Dashboard v7.0 ===');
    showLoadingSkeleton();
    try {
        let lines = null, sourceUsed = null;
        const errors = [];
        for (const source of DATA_SOURCES) {
            try {
                lines = await tryLoadFromSource(source);
                if (lines && lines.length > 0) { sourceUsed = source; break; }
            } catch (err) { errors.push(`${source}: ${err.message}`); }
        }
        if (!lines || lines.length === 0) {
            // Use embedded inline fallback data
            console.warn('All fetch sources failed, using inline fallback data');
            lines = INLINE_FALLBACK_JSONL;
            sourceUsed = 'inline-fallback';
            if (!lines || lines.length === 0) {
                throw new Error(`No benchmark data found.\nTried:\n${errors.join('\n')}\n\nRun: .\\scripts\\run_benchmark_dashboard.ps1`);
            }
        }

        benchmarkData = lines.map(line => {
            const entry = JSON.parse(line);
            entry.timestamp = new Date(entry.timestamp);
            if (!entry.hardware_id) entry.hardware_id = BASELINE_HARDWARE_ID;
            if (!entry.hardware_label) entry.hardware_label = BASELINE_HARDWARE_LABEL;
            return entry;
        });

        // Deduplicate
        const dedupeMap = new Map();
        benchmarkData.forEach(e => {
            const key = `${e.benchmark_name}_${e.timestamp.getTime()}_${e.hardware_id}`;
            if (!dedupeMap.has(key)) dedupeMap.set(key, e);
        });
        benchmarkData = Array.from(dedupeMap.values());
        benchmarkData.sort((a, b) => a.timestamp - b.timestamp);

        benchmarkData.forEach(d => {
            if (!hardwareProfiles.has(d.hardware_id))
                hardwareProfiles.set(d.hardware_id, d.hardware_label);
        });

        console.log(`Loaded: ${benchmarkData.length} entries, ${hardwareProfiles.size} hardware(s) from ${sourceUsed}`);
        updateFilters();
        restoreFiltersFromURL();
        autoAdjustTimeRange();
        renderDashboard();
        const loadTime = (performance.now() - PERF.loadStart).toFixed(0);
        console.log(`Dashboard ready in ${loadTime}ms`);
        const ltEl = document.getElementById('load-time');
        if (ltEl) ltEl.textContent = `Loaded in ${loadTime}ms`;
    } catch (error) {
        console.error(error);
        showError(error.message);
    }
}

// ─── FILTERS ─────────────────────────────────────────────────────────────────

function updateFilters() {
    const benchmarkSelect = document.getElementById('benchmark-select');
    const benchMap = new Map();
    benchmarkData.forEach(d => {
        if (!benchMap.has(d.benchmark_name))
            benchMap.set(d.benchmark_name, d.display_name || d.benchmark_name);
    });
    const sorted = Array.from(benchMap.entries())
        .map(([n, d]) => ({ name: n, display: d }))
        .sort((a, b) => a.display.localeCompare(b.display));
    benchmarkSelect.innerHTML = '<option value="all">All Benchmarks</option>';
    sorted.forEach(b => {
        const o = document.createElement('option');
        o.value = b.name; o.textContent = b.display;
        benchmarkSelect.appendChild(o);
    });

    const hwSelect = document.getElementById('hardware-filter');
    if (hwSelect) {
        hwSelect.innerHTML = '<option value="all">All Hardware</option>';
        hwSelect.innerHTML += `<option value="${BASELINE_HARDWARE_ID}">Baseline (HP Pavilion)</option>`;
        hardwareProfiles.forEach((label, id) => {
            if (id !== BASELINE_HARDWARE_ID) {
                const o = document.createElement('option');
                o.value = id; o.textContent = `User: ${label.substring(0, 50)}`;
                hwSelect.appendChild(o);
            }
        });
    }

    document.getElementById('system-filter').addEventListener('change', onFilterChange);
    document.getElementById('time-range').addEventListener('change', onFilterChange);
    document.getElementById('benchmark-select').addEventListener('change', onFilterChange);
    if (hwSelect) hwSelect.addEventListener('change', onFilterChange);
    const toggle = document.getElementById('industry-toggle');
    if (toggle) toggle.addEventListener('change', e => { currentFilters.showIndustry = e.target.checked; debouncedRender(); });
}

function onFilterChange() {
    currentFilters.system = document.getElementById('system-filter').value;
    currentFilters.timeRange = document.getElementById('time-range').value;
    currentFilters.benchmark = document.getElementById('benchmark-select').value;
    const hw = document.getElementById('hardware-filter');
    if (hw) currentFilters.hardware = hw.value;
    persistFiltersToURL();
    debouncedRender();
}

// ─── URL STATE ───────────────────────────────────────────────────────────────

function persistFiltersToURL() {
    const p = new URLSearchParams();
    if (currentFilters.system !== 'all') p.set('sys', currentFilters.system);
    if (currentFilters.timeRange !== 'all') p.set('t', currentFilters.timeRange);
    if (currentFilters.benchmark && currentFilters.benchmark !== 'all') p.set('b', currentFilters.benchmark);
    if (currentFilters.hardware !== 'all') p.set('hw', currentFilters.hardware);
    if (!currentFilters.showIndustry) p.set('ind', '0');
    const qs = p.toString();
    history.replaceState(null, '', qs ? '?' + qs : location.pathname);
}

function restoreFiltersFromURL() {
    const p = new URLSearchParams(location.search);
    if (p.has('sys')) { currentFilters.system = p.get('sys'); document.getElementById('system-filter').value = currentFilters.system; }
    if (p.has('t')) { currentFilters.timeRange = p.get('t'); document.getElementById('time-range').value = currentFilters.timeRange; }
    if (p.has('b')) { currentFilters.benchmark = p.get('b'); document.getElementById('benchmark-select').value = currentFilters.benchmark; }
    if (p.has('hw')) { currentFilters.hardware = p.get('hw'); const el = document.getElementById('hardware-filter'); if (el) el.value = currentFilters.hardware; }
    if (p.get('ind') === '0') { currentFilters.showIndustry = false; const el = document.getElementById('industry-toggle'); if (el) el.checked = false; }
}

// Auto-adjust time range: if the default/selected range yields 0 results,
// progressively widen until data is found, then fall back to "All Time".
function autoAdjustTimeRange() {
    const now = new Date();
    const tr = currentFilters.timeRange;
    if (tr === 'all') return; // already showing everything

    const cutoff = new Date(now.getTime() - parseInt(tr) * 86400000);
    const hasData = benchmarkData.some(d => d.timestamp >= cutoff);
    if (hasData) return; // current range has data

    // Try progressively wider ranges
    const wider = ['30', '60', '90', '180', '365', 'all'];
    for (const w of wider) {
        if (w === 'all') {
            currentFilters.timeRange = 'all';
            break;
        }
        const wCutoff = new Date(now.getTime() - parseInt(w) * 86400000);
        if (benchmarkData.some(d => d.timestamp >= wCutoff)) {
            currentFilters.timeRange = w;
            break;
        }
    }
    // Update the dropdown to reflect the auto-adjusted value
    const sel = document.getElementById('time-range');
    if (sel) {
        // If the computed range isn't an option, fall back to "all"
        const opts = Array.from(sel.options).map(o => o.value);
        if (!opts.includes(currentFilters.timeRange)) currentFilters.timeRange = 'all';
        sel.value = currentFilters.timeRange;
    }
    console.log(`Auto-adjusted time range to '${currentFilters.timeRange}' (original '${tr}' had no data)`);
}

function applyFilters() {
    const now = new Date();
    let cutoff = null;
    if (currentFilters.timeRange !== 'all') {
        cutoff = new Date(now.getTime() - parseInt(currentFilters.timeRange) * 86400000);
    }
    filteredData = benchmarkData.filter(d => {
        if (cutoff && d.timestamp < cutoff) return false;
        if (currentFilters.system !== 'all' && detectSystem(d.crate) !== currentFilters.system) return false;
        if (currentFilters.benchmark && currentFilters.benchmark !== 'all' && d.benchmark_name !== currentFilters.benchmark) return false;
        if (currentFilters.hardware !== 'all' && d.hardware_id !== currentFilters.hardware) return false;
        return true;
    });
}

// ─── SYSTEM DETECTION ────────────────────────────────────────────────────────

function detectSystem(crate) {
    const c = (crate || '').toLowerCase();
    if (c.includes('animation') || c.includes('crossfade') || c.includes('joint_palette') ||
        c.includes('keyframe') || c.match(/blend.*mode/)) return 'animation';
    if (c.includes('core') || c.includes('ecs') || c.includes('stress') || c.includes('entity') ||
        c.includes('archetype') || c.includes('query') || (c.includes('world') && !c.includes('chunk') && !c.includes('partition')))
        return 'ecs';
    if (c.includes('ai') || c.includes('behavior') || c.includes('llm') || c.includes('context') ||
        c.includes('memory') || c.includes('persona') || c.includes('rag') || c.includes('prompts') ||
        c.includes('goap') || c.includes('utility'))
        return 'ai';
    if (c.includes('physics') || c.includes('nav') || c.includes('character_controller') ||
        c.includes('raycast') || c.includes('collision') || c.includes('spatial_hash'))
        return 'physics';
    if (c.includes('terrain') || c.includes('pcg') || c.includes('weaving') || c.includes('heightmap') ||
        c.includes('voxel') || c.includes('chunk') || c.includes('erosion') || c.includes('climate') ||
        c.includes('worldpartition') || c.includes('world_chunk'))
        return 'terrain';
    if (c.includes('render') || c.includes('culling') || c.includes('shader') || c.includes('texture') ||
        c.includes('mesh') || c.includes('cluster') || c.includes('lod') || c.includes('instanc') ||
        c.includes('bloom') || c.includes('shadow') || c.includes('light') || c.includes('hiz') ||
        c.includes('ibl') || c.includes('msaa') || c.includes('decal') || c.includes('depth') ||
        c.includes('frustum') || c.includes('aabb') || c.includes('camera') || c.includes('csm') ||
        c.includes('cascade') || c.includes('gpu') || c.includes('overlay') || c.includes('pcf') ||
        c.includes('prefix_sum') || c.includes('indirect') || c.includes('material') || c.includes('bake') ||
        c.includes('baking'))
        return 'rendering';
    if (c.includes('math') || c.includes('vec') || c.includes('mat') || c.includes('simd') || c.includes('quat'))
        return 'math';
    if (c.includes('net') || c.includes('network')) return 'networking';
    if (c.includes('persistence') || c.includes('save')) return 'persistence';
    if (c.includes('input')) return 'input';
    if (c.includes('audio')) return 'audio';
    if (c.includes('ui') || c.includes('gizmo') || c.includes('widget') || c.includes('astract'))
        return 'ui';
    if (c.includes('sdk') || c.includes('editor') || c.includes('build') || c.includes('hash'))
        return 'tools';
    if (c.includes('enemy') || c.includes('player') || c.includes('quest') || c.includes('attack') ||
        c.includes('combat') || c.includes('integrated') || c.includes('battle') || c.includes('pattern') ||
        c.includes('weather') || c.includes('fluids'))
        return 'ecs';
    return 'other';
}

function getSystemColor(crate) {
    return COLOR_SCHEME[detectSystem(crate)] || COLOR_SCHEME.default;
}

// ─── INDUSTRY STANDARD LOOKUP ────────────────────────────────────────────────

function findIndustryStandard(benchmarkName, system) {
    const stds = INDUSTRY_STANDARDS[system];
    if (!stds) return null;
    const lower = benchmarkName.toLowerCase();
    for (const [key, engines] of Object.entries(stds.benchmarks)) {
        if (lower.includes(key.toLowerCase())) {
            const vals = Object.values(engines);
            const avg = vals.reduce((a, b) => a + b, 0) / vals.length;
            return { average: avg, engines, key };
        }
    }
    return null;
}

// ─── RENDER DASHBOARD ────────────────────────────────────────────────────────

function renderDashboard() {
    const renderStart = performance.now();
    try {
        applyFilters();
        renderHardwareComparisonBar();
        renderProductionHealthSummary();
        renderRegressionAlerts();
        renderStatCards();
        renderChart();
        renderIndustryComparisonSection();
        renderBenchTable();
        updateGeneratedTime();
        PERF.renderCount++;
        const elapsed = (performance.now() - renderStart).toFixed(0);
        console.log(`Render #${PERF.renderCount} in ${elapsed}ms (${filteredData.length} pts)`);
    } catch (error) {
        console.error('Error rendering:', error);
        showError(`Rendering error: ${error.message}`);
    }
}

// ─── HARDWARE COMPARISON BAR ─────────────────────────────────────────────────

function renderHardwareComparisonBar() {
    const c = document.getElementById('hardware-info');
    if (!c) return;
    const baseCount = benchmarkData.filter(d => d.hardware_id === BASELINE_HARDWARE_ID).length;
    const userCount = benchmarkData.filter(d => d.hardware_id !== BASELINE_HARDWARE_ID).length;

    let baseInfo = `<span style="color:#43e97b">Baseline: ${BASELINE_HARDWARE_LABEL}</span>`;
    let userInfo = '';
    const userHw = Array.from(hardwareProfiles.entries()).filter(([id]) => id !== BASELINE_HARDWARE_ID);
    if (userHw.length > 0) {
        userInfo = userHw.map(([, label]) => `<span style="color:#4facfe">User: ${label}</span>`).join('<br/>');
    }

    c.innerHTML = `
        <div style="display:flex;gap:24px;align-items:center;flex-wrap:wrap;">
            <div style="flex:1;min-width:200px;">
                <div style="font-size:0.85em;color:#a0a0a0;margin-bottom:4px;">Hardware Profiles (${hardwareProfiles.size})</div>
                <div style="font-size:0.9em;">${baseInfo}</div>
                ${userInfo ? `<div style="font-size:0.9em;margin-top:4px;">${userInfo}</div>` : ''}
            </div>
            <div style="display:flex;gap:16px;align-items:center;">
                <div style="text-align:center;padding:8px 16px;background:rgba(67,233,123,0.1);border-radius:8px;">
                    <div style="font-size:1.4em;font-weight:bold;color:#43e97b;">${baseCount}</div>
                    <div style="font-size:0.75em;color:#a0a0a0;">Baseline Points</div>
                </div>
                <div style="text-align:center;padding:8px 16px;background:rgba(79,172,254,0.1);border-radius:8px;">
                    <div style="font-size:1.4em;font-weight:bold;color:#4facfe;">${userCount}</div>
                    <div style="font-size:0.75em;color:#a0a0a0;">User Points</div>
                </div>
            </div>
        </div>`;
}

// ─── STAT CARDS ──────────────────────────────────────────────────────────────

function renderStatCards() {
    const grid = document.getElementById('stats-grid');
    if (filteredData.length === 0) {
        grid.innerHTML = '<div class="loading">No data for selected filters</div>';
        return;
    }
    const groups = {};
    filteredData.forEach(d => { (groups[d.benchmark_name] = groups[d.benchmark_name] || []).push(d); });

    const stats = [];
    for (const [name, entries] of Object.entries(groups)) {
        entries.sort((a, b) => a.timestamp - b.timestamp);
        const latest = entries[entries.length - 1], oldest = entries[0];
        const change = entries.length > 1 ? ((latest.value - oldest.value) / oldest.value) * 100 : 0;
        const sys = detectSystem(latest.crate);
        const ref = findIndustryStandard(name, sys);
        stats.push({
            name, value: latest.value, unit: latest.unit, change, system: sys,
            color: getSystemColor(latest.crate),
            industryAvg: ref ? ref.average : null,
            isBaseline: latest.hardware_id === BASELINE_HARDWARE_ID
        });
    }
    stats.sort((a, b) => b.value - a.value);

    grid.innerHTML = '';
    stats.slice(0, 8).forEach(s => {
        const card = document.createElement('div');
        card.className = `stat-card ${s.system}`;
        const icon = s.change > 0 ? '▲' : s.change < 0 ? '▼' : '─';
        const cc = s.change > 10 ? '#ff6b6b' : s.change < -10 ? '#43e97b' : '#a0a0a0';
        let badge = '';
        if (s.industryAvg !== null && currentFilters.showIndustry) {
            const ratio = ((s.value / s.industryAvg) * 100).toFixed(0);
            const ic = s.value <= s.industryAvg ? '#43e97b' : '#ff6b6b';
            const il = s.value <= s.industryAvg ? 'faster' : 'slower';
            badge = `<div style="font-size:0.75em;color:${ic};margin-top:4px;">vs Industry: ${ratio}% (${il})</div>`;
        }
        card.innerHTML = `
            <div class="stat-label">${s.name}</div>
            <div class="stat-value">${formatNumber(s.value)}<span class="stat-unit">${s.unit}</span></div>
            <div class="stat-change" style="color:${cc}">${icon} ${Math.abs(s.change).toFixed(1)}%
                ${s.isBaseline ? '<span style="font-size:0.7em;color:#43e97b;margin-left:4px;">[baseline]</span>' : ''}
            </div>${badge}`;
        grid.appendChild(card);
    });
}

function formatNumber(v) {
    if (v === 0) return '0';
    if (Math.abs(v) < 0.01) return v.toExponential(1);
    if (Math.abs(v) < 10) return v.toFixed(2);
    if (Math.abs(v) < 100) return v.toFixed(1);
    if (Math.abs(v) < 1000) return v.toFixed(0);
    if (Math.abs(v) < 1e6) return (v / 1e3).toFixed(1) + 'K';
    if (Math.abs(v) < 1e9) return (v / 1e6).toFixed(1) + 'M';
    return (v / 1e9).toFixed(2) + 'G';
}

function formatDuration(ns) {
    if (ns < 1000) return ns.toFixed(1) + ' ns';
    if (ns < 1e6) return (ns / 1e3).toFixed(1) + ' µs';
    if (ns < 1e9) return (ns / 1e6).toFixed(1) + ' ms';
    return (ns / 1e9).toFixed(2) + ' s';
}

// ─── MAIN CHART ──────────────────────────────────────────────────────────────

function renderChart() {
    const div = document.getElementById('chart');
    div.innerHTML = '';
    if (filteredData.length === 0) { div.innerHTML = '<div class="loading">No data</div>'; return; }
    if (typeof d3 === 'undefined') {
        div.innerHTML = '<div class="loading" style="color:#ff6b6b;">D3.js failed to load.</div>';
        return;
    }

    const benchGroups = d3.group(filteredData, d => d.benchmark_name);
    let series = [];
    benchGroups.forEach((values, name) => {
        values.sort((a, b) => a.timestamp - b.timestamp);
        const hwGroups = d3.group(values, d => d.hardware_id);
        hwGroups.forEach((hwVals, hwId) => {
            const isBase = hwId === BASELINE_HARDWARE_ID;
            series.push({
                name: hardwareProfiles.size > 1 ? `${name} [${isBase ? 'Baseline' : 'User'}]` : name,
                rawName: name,
                values: hwVals,
                color: getSystemColor(hwVals[0].crate),
                isBaseline: isBase,
                hardwareId: hwId
            });
        });
    });

    let chartSeries = series;
    if (currentFilters.benchmark && currentFilters.benchmark !== 'all')
        chartSeries = series.filter(s => s.rawName === currentFilters.benchmark);

    const title = document.getElementById('chart-title');

    // Limit series when showing "all" to keep the chart readable
    const MAX_VISIBLE_SERIES = 25;
    let hiddenCount = 0;
    if ((!currentFilters.benchmark || currentFilters.benchmark === 'all') && chartSeries.length > MAX_VISIBLE_SERIES) {
        // Sort by latest value descending and keep top N
        chartSeries.sort((a, b) => {
            const aLast = a.values[a.values.length - 1]?.value || 0;
            const bLast = b.values[b.values.length - 1]?.value || 0;
            return bLast - aLast;
        });
        hiddenCount = chartSeries.length - MAX_VISIBLE_SERIES;
        chartSeries = chartSeries.slice(0, MAX_VISIBLE_SERIES);
    }

    const allVals = chartSeries.flatMap(s => s.values);
    const allTimestamps = [...new Set(allVals.map(d => d.timestamp.getTime()))];
    const isSinglePoint = allTimestamps.length <= 1;

    let yMax = d3.max(allVals, d => d.value) * 1.15;

    // Industry lines
    const industryLines = [];
    if (currentFilters.showIndustry) {
        const seen = new Set();
        chartSeries.forEach(s => {
            if (seen.has(s.rawName)) return;
            seen.add(s.rawName);
            const sys = detectSystem(s.values[0].crate);
            const ref = findIndustryStandard(s.rawName, sys);
            if (ref) {
                industryLines.push({ name: s.rawName, value: ref.average, engines: ref.engines, color: INDUSTRY_LINE_COLOR });
                if (ref.average * 1.1 > yMax) yMax = ref.average * 1.15;
            }
        });
    }

    // Auto-detect if log scale is needed (value range > 100×)
    const allValsPositive = allVals.filter(d => d.value > 0);
    const valMin = allValsPositive.length > 0 ? d3.min(allValsPositive, d => d.value) : 1;
    const valMax = d3.max(allVals, d => d.value) || 1;
    const useLog = (valMax / Math.max(valMin, 1e-10)) > 100;

    title.textContent = (currentFilters.benchmark && currentFilters.benchmark !== 'all')
        ? currentFilters.benchmark
        : isSinglePoint
            ? `Single Snapshot — ${chartSeries.length} Benchmarks (grouped by magnitude)`
            : hiddenCount > 0
                ? `Top ${MAX_VISIBLE_SERIES} Benchmarks by Value (${hiddenCount} more hidden — use filters)`
                : `All Benchmarks (${chartSeries.length} series)`;

    // ── SINGLE-SNAPSHOT: render bar chart instead of line chart ──
    if (isSinglePoint) {
        renderSingleSnapshotBars(div, chartSeries, industryLines, useLog, yMax, valMin);
        renderLegend(chartSeries, industryLines);
        renderHistogram(chartSeries);
        renderSparklines(chartSeries);
        return;
    }

    // ── MULTI-POINT LINE CHART ──
    const margin = { top: 24, right: 160, bottom: 70, left: 100 };
    const width = div.clientWidth - margin.left - margin.right;
    const height = 620 - margin.top - margin.bottom;

    const svg = d3.select(div).append('svg')
        .attr('width', width + margin.left + margin.right)
        .attr('height', height + margin.top + margin.bottom)
        .append('g').attr('transform', `translate(${margin.left},${margin.top})`);

    const xScale = d3.scaleTime().domain(d3.extent(allVals, d => d.timestamp)).range([0, width]);
    const yScale = useLog
        ? d3.scaleSymlog().constant(Math.max(valMin * 0.1, 0.1)).domain([0, yMax]).range([height, 0])
        : d3.scaleLinear().domain([0, yMax]).range([height, 0]);

    // Grid
    svg.append('g').attr('class', 'grid').attr('transform', `translate(0,${height})`)
        .call(d3.axisBottom(xScale).tickSize(-height).tickFormat(''));
    svg.append('g').attr('class', 'grid').call(d3.axisLeft(yScale).tickSize(-width).tickFormat(''));

    // Axes
    svg.append('g').attr('class', 'axis').attr('transform', `translate(0,${height})`)
        .call(d3.axisBottom(xScale).ticks(8).tickFormat(d3.timeFormat('%b %d')))
        .selectAll('text').style('font-size', '13px').attr('fill', '#c0c0c0');
    const yAxisGen = useLog
        ? d3.axisLeft(yScale).ticks(10).tickFormat(d => d === 0 ? '0' : formatNumber(d))
        : d3.axisLeft(yScale).ticks(8).tickFormat(d => formatNumber(d));
    svg.append('g').attr('class', 'axis').call(yAxisGen)
        .selectAll('text').style('font-size', '13px').attr('fill', '#c0c0c0');
    svg.append('text').attr('transform', 'rotate(-90)').attr('y', -75).attr('x', -height / 2)
        .attr('text-anchor', 'middle').attr('fill', '#b0b0b0').style('font-size', '15px').style('font-weight', '500')
        .text(useLog ? 'Value (log scale)' : 'Time (ns)');
    svg.append('text').attr('x', width / 2).attr('y', height + 55)
        .attr('text-anchor', 'middle').attr('fill', '#b0b0b0').style('font-size', '14px').text('Date');

    // Industry standard lines
    industryLines.forEach(il => {
        svg.append('line').attr('x1', 0).attr('x2', width)
            .attr('y1', yScale(il.value)).attr('y2', yScale(il.value))
            .attr('stroke', il.color).attr('stroke-width', 2)
            .attr('stroke-dasharray', '8,4').attr('opacity', 0.7);
        const names = Object.keys(il.engines).join('/');
        svg.append('text').attr('x', width + 4).attr('y', yScale(il.value) + 4)
            .attr('fill', il.color).style('font-size', '10px').text(`Industry (${names})`);
    });

    // Data lines
    const lineGen = d3.line().x(d => xScale(d.timestamp)).y(d => yScale(d.value)).curve(d3.curveMonotoneX);
    chartSeries.forEach(s => {
        svg.append('path').datum(s.values).attr('class', 'line').attr('d', lineGen)
            .attr('stroke', s.color).attr('stroke-width', s.isBaseline ? 3.5 : 2.5)
            .attr('stroke-dasharray', s.isBaseline ? 'none' : '8,4').attr('opacity', s.isBaseline ? 1 : 0.85);
        svg.selectAll(null).data(s.values).enter().append('circle')
            .attr('cx', d => xScale(d.timestamp)).attr('cy', d => yScale(d.value))
            .attr('r', 4).attr('fill', s.color).attr('opacity', 0.8);
    });

    // Tooltip
    const tooltip = d3.select('.tooltip');
    svg.append('rect').attr('width', width).attr('height', height)
        .attr('fill', 'none').attr('pointer-events', 'all')
        .on('mousemove', function(event) {
            const [mx] = d3.pointer(event);
            const x0 = xScale.invert(mx);
            let closest = null, closestD = Infinity;
            allVals.forEach(d => { const dist = Math.abs(d.timestamp - x0); if (dist < closestD) { closestD = dist; closest = d; } });
            if (closest) {
                const isB = closest.hardware_id === BASELINE_HARDWARE_ID;
                const sys = detectSystem(closest.crate);
                const ref = findIndustryStandard(closest.benchmark_name, sys);
                let indLine = '';
                if (ref && currentFilters.showIndustry) {
                    const ratio = ((closest.value / ref.average) * 100).toFixed(0);
                    indLine = `<br/><span style="color:${INDUSTRY_LINE_COLOR}">Industry avg: ${formatDuration(ref.average)} (${ratio}%)</span>`;
                }
                const sdLine = closest.stddev ? `<br/>\u00b1${formatNumber(closest.stddev)} (\u03c3)` : '';
                tooltip.style('left', (event.pageX + 15) + 'px').style('top', (event.pageY - 28) + 'px')
                    .classed('visible', true)
                    .html(`<strong>${closest.display_name || closest.benchmark_name}</strong><br/>
                        Value: ${formatDuration(closest.value)}${sdLine}<br/>
                        Date: ${d3.timeFormat('%Y-%m-%d %H:%M')(closest.timestamp)}<br/>
                        Hardware: ${isB ? '🏠 Baseline' : '🖥️ User'}<br/>
                        Branch: ${closest.git_branch} | SHA: ${closest.git_sha}${indLine}`);
            }
        })
        .on('mouseleave', () => tooltip.classed('visible', false));

    renderLegend(chartSeries, industryLines);
    renderHistogram(chartSeries);
    renderSparklines(chartSeries);
}

// ─── TIERED SINGLE-SNAPSHOT BAR CHART ─────────────────────────────────────────
// Renders benchmarks grouped by magnitude tier (ns / µs / ms / s), each with
// its own independent linear scale.  This makes all bars readable regardless
// of the data spanning many orders of magnitude (e.g. 12 ns → 4.4 ms).

function renderSingleSnapshotBars(div, chartSeries, industryLines, useLog, yMax, valMin) {
    // ── 1. Collect items ────────────────────────────────────────────────────
    const items = chartSeries.map(s => {
        const latest = s.values[s.values.length - 1];
        return {
            name: (latest.display_name || s.rawName).substring(0, 50),
            value: latest.value,
            unit: latest.unit,
            color: s.color,
            isBaseline: s.isBaseline,
            hardwareId: s.hardwareId,
            crate: latest.crate,
            rawName: s.rawName,
            stddev: latest.stddev
        };
    });

    // ── 2. Build magnitude tiers ────────────────────────────────────────────
    const tierDefs = [
        { key: 'ns',  label: 'Nanosecond Scale',  unit: 'ns', lo: 0,    hi: 1e3,  divisor: 1,   fmt: 'ns' },
        { key: 'us',  label: 'Microsecond Scale',  unit: 'µs', lo: 1e3,  hi: 1e6,  divisor: 1e3, fmt: 'µs' },
        { key: 'ms',  label: 'Millisecond Scale',  unit: 'ms', lo: 1e6,  hi: 1e9,  divisor: 1e6, fmt: 'ms' },
        { key: 's',   label: 'Second Scale',        unit: 's',  lo: 1e9,  hi: Infinity, divisor: 1e9, fmt: 's'  }
    ];

    const tiers = tierDefs.map(td => ({
        ...td,
        items: items.filter(it => it.value >= td.lo && it.value < td.hi)
                     .sort((a, b) => b.value - a.value)
    })).filter(t => t.items.length > 0);

    // ── 3. Layout constants ─────────────────────────────────────────────────
    const labelWidth   = 240;
    const barHeight    = 24;
    const gap          = 4;
    const tierGap      = 28;          // vertical space between tier sections
    const tierHeaderH  = 32;          // height of tier header row
    const tierAxisH    = 36;          // space for each tier's X-axis
    const margin       = { top: 16, right: 120, bottom: 24, left: labelWidth + 20 };
    const totalWidth   = div.clientWidth || 800;
    const barAreaWidth = totalWidth - margin.left - margin.right;

    // Pre-calculate total SVG height
    let totalHeight = margin.top;
    tiers.forEach((t, ti) => {
        totalHeight += tierHeaderH;
        totalHeight += (barHeight + gap) * t.items.length;
        totalHeight += tierAxisH;
        if (ti < tiers.length - 1) totalHeight += tierGap;
    });
    totalHeight += margin.bottom + 24; // extra for legend

    // ── 4. Create SVG ───────────────────────────────────────────────────────
    const svg = d3.select(div).append('svg')
        .attr('width', totalWidth)
        .attr('height', totalHeight)
        .append('g').attr('transform', `translate(${margin.left},${margin.top})`);

    const tooltip = d3.select('.tooltip');
    const indMap = new Map();
    industryLines.forEach(il => indMap.set(il.name, il));

    // Tier accent colors (subtle left-border accent)
    const tierColors = { ns: '#43e97b', us: '#38f9d7', ms: '#f7971e', s: '#ff6b6b' };

    // ── 5. Render each tier ─────────────────────────────────────────────────
    let cursorY = 0;

    tiers.forEach((tier, ti) => {
        const tierMaxRaw = d3.max(tier.items, d => d.value) || 1;
        const tierMax    = tierMaxRaw * 1.15;                     // 15% headroom
        const scaledMax  = tierMax / tier.divisor;

        // Independent linear scale for this tier (in tier-native units)
        const xScale = d3.scaleLinear().domain([0, scaledMax]).range([0, barAreaWidth]);

        // ── Tier header ─────────────────────────────────────────────────────
        const accentColor = tierColors[tier.key] || '#ccc';

        // Accent bar on left
        svg.append('rect')
            .attr('x', -labelWidth - 16).attr('y', cursorY)
            .attr('width', 4).attr('height', tierHeaderH - 4)
            .attr('fill', accentColor).attr('rx', 2);

        // Tier title
        svg.append('text')
            .attr('x', -labelWidth - 6).attr('y', cursorY + tierHeaderH / 2 + 5)
            .attr('fill', accentColor).style('font-size', '14px').style('font-weight', '700')
            .style('letter-spacing', '0.5px')
            .text(`${tier.label}  (${tier.items.length})`);

        // Subtle horizontal rule
        svg.append('line')
            .attr('x1', -labelWidth - 16).attr('x2', barAreaWidth)
            .attr('y1', cursorY + tierHeaderH - 2).attr('y2', cursorY + tierHeaderH - 2)
            .attr('stroke', 'rgba(255,255,255,0.08)').attr('stroke-width', 1);

        cursorY += tierHeaderH;

        // ── Bars for this tier ──────────────────────────────────────────────
        tier.items.forEach((item, idx) => {
            const y = cursorY + idx * (barHeight + gap);
            const scaledVal = item.value / tier.divisor;
            const barW = Math.max(xScale(scaledVal), 3);

            // Bar
            svg.append('rect')
                .attr('x', 0).attr('y', y)
                .attr('width', barW).attr('height', barHeight)
                .attr('fill', item.color).attr('opacity', 0.85).attr('rx', 3)
                .on('mouseenter', function(event) {
                    d3.select(this).attr('opacity', 1).attr('stroke', '#fff').attr('stroke-width', 1);
                    const sys = detectSystem(item.crate);
                    const ref = findIndustryStandard(item.rawName, sys);
                    let indLine = '';
                    if (ref && currentFilters.showIndustry) {
                        const ratio = ((item.value / ref.average) * 100).toFixed(0);
                        indLine = `<br/><span style="color:${INDUSTRY_LINE_COLOR}">Industry avg: ${formatDuration(ref.average)} (${ratio}%)</span>`;
                    }
                    const sdLine = item.stddev ? `<br/>\u00b1${formatNumber(item.stddev)} (\u03c3)` : '';
                    tooltip.style('left', (event.pageX + 15) + 'px').style('top', (event.pageY - 28) + 'px')
                        .classed('visible', true)
                        .html(`<strong>${item.name}</strong><br/>Value: ${formatDuration(item.value)}${sdLine}<br/>Tier: ${tier.label}<br/>Hardware: ${item.isBaseline ? '🏠 Baseline' : '🖥️ User'}${indLine}`);
                })
                .on('mouseleave', function() {
                    d3.select(this).attr('opacity', 0.85).attr('stroke', 'none');
                    tooltip.classed('visible', false);
                });

            // Industry standard marker (vertical line)
            const ind = indMap.get(item.rawName);
            if (ind && currentFilters.showIndustry) {
                const indScaled = ind.value / tier.divisor;
                if (indScaled <= scaledMax) {
                    const indX = xScale(indScaled);
                    svg.append('line')
                        .attr('x1', indX).attr('x2', indX)
                        .attr('y1', y - 2).attr('y2', y + barHeight + 2)
                        .attr('stroke', INDUSTRY_LINE_COLOR).attr('stroke-width', 2.5)
                        .attr('opacity', 0.9);
                }
            }

            // Value label (right of bar)
            svg.append('text')
                .attr('x', barW + 8).attr('y', y + barHeight / 2 + 4)
                .attr('fill', '#c0c0c0').style('font-size', '11px').style('font-weight', '500')
                .text(formatDuration(item.value));

            // Benchmark name label (left of bar)
            svg.append('text')
                .attr('x', -8).attr('y', y + barHeight / 2 + 4)
                .attr('text-anchor', 'end').attr('fill', '#e0e0e0')
                .style('font-size', '12px').style('font-weight', '400')
                .text(item.name);
        });

        cursorY += (barHeight + gap) * tier.items.length;

        // ── Tier X-axis (in native units) ───────────────────────────────────
        const xAxisGen = d3.axisBottom(xScale).ticks(6)
            .tickFormat(d => d === 0 ? '0' : d3.format('.4~g')(d) + ' ' + tier.fmt);
        svg.append('g').attr('class', 'axis')
            .attr('transform', `translate(0,${cursorY + 4})`)
            .call(xAxisGen)
            .selectAll('text').style('font-size', '11px').attr('fill', '#999');

        // Grid lines for this tier
        svg.append('g').attr('class', 'grid')
            .attr('transform', `translate(0,${cursorY + 4})`)
            .call(d3.axisBottom(xScale)
                .tickSize(-(barHeight + gap) * tier.items.length - 4)
                .tickFormat(''))
            .selectAll('line').attr('stroke', 'rgba(255,255,255,0.04)');

        cursorY += tierAxisH;
        if (ti < tiers.length - 1) cursorY += tierGap;
    });

    // ── 6. Industry legend ──────────────────────────────────────────────────
    if (industryLines.length > 0 && currentFilters.showIndustry) {
        svg.append('line').attr('x1', 0).attr('x2', 20)
            .attr('y1', cursorY + 8).attr('y2', cursorY + 8)
            .attr('stroke', INDUSTRY_LINE_COLOR).attr('stroke-width', 2.5);
        svg.append('text').attr('x', 26).attr('y', cursorY + 12)
            .attr('fill', INDUSTRY_LINE_COLOR).style('font-size', '11px').text('Industry Standard');
    }
}

// ─── INDUSTRY COMPARISON SECTION ─────────────────────────────────────────────

function renderIndustryComparisonSection() {
    const c = document.getElementById('industry-comparison');
    if (!c) return;
    if (!currentFilters.showIndustry) { c.innerHTML = ''; return; }

    const groups = {};
    filteredData.forEach(d => { (groups[d.benchmark_name] = groups[d.benchmark_name] || []).push(d); });

    const comparisons = [];
    for (const [name, entries] of Object.entries(groups)) {
        entries.sort((a, b) => a.timestamp - b.timestamp);
        const latest = entries[entries.length - 1];
        const sys = detectSystem(latest.crate);
        const ref = findIndustryStandard(name, sys);
        if (ref) comparisons.push({
            name, displayName: latest.display_name || name,
            ourValue: latest.value, unit: latest.unit, system: sys,
            engines: ref.engines, industryAvg: ref.average
        });
    }

    if (comparisons.length === 0) {
        c.innerHTML = '<div class="loading" style="font-size:1em;">No industry comparisons for current filters</div>';
        return;
    }
    comparisons.sort((a, b) => (a.ourValue / a.industryAvg) - (b.ourValue / b.industryAvg));

    const margin = { top: 24, right: 30, bottom: 50, left: 300 };
    const barH = 36, gap = 8;
    const totalH = (barH + gap) * comparisons.length + margin.top + margin.bottom;
    const svgW = c.clientWidth || 800;
    const w = svgW - margin.left - margin.right;

    c.innerHTML = '';
    const svg = d3.select(c).append('svg').attr('width', svgW).attr('height', totalH)
        .append('g').attr('transform', `translate(${margin.left},${margin.top})`);

    const maxV = d3.max(comparisons, d => Math.max(d.ourValue, d.industryAvg)) * 1.2;
    const minVInd = d3.min(comparisons.filter(d => d.ourValue > 0 && d.industryAvg > 0), d => Math.min(d.ourValue, d.industryAvg)) || 1;
    const indUseLog = (maxV / Math.max(minVInd, 1e-10)) > 100;
    const x = indUseLog
        ? d3.scaleSymlog().constant(Math.max(minVInd * 0.1, 0.1)).domain([0, maxV]).range([0, w])
        : d3.scaleLinear().domain([0, maxV]).range([0, w]);

    comparisons.forEach((comp, i) => {
        const y = i * (barH + gap);
        // Industry bar
        svg.append('rect').attr('x', 0).attr('y', y).attr('width', x(comp.industryAvg))
            .attr('height', barH / 2).attr('fill', INDUSTRY_LINE_COLOR).attr('opacity', 0.4).attr('rx', 3);
        // Our bar
        const col = comp.ourValue <= comp.industryAvg ? '#43e97b' : '#ff6b6b';
        svg.append('rect').attr('x', 0).attr('y', y + barH / 2).attr('width', x(comp.ourValue))
            .attr('height', barH / 2).attr('fill', col).attr('opacity', 0.8).attr('rx', 3);
        // Label
        svg.append('text').attr('x', -10).attr('y', y + barH / 2 + 3).attr('text-anchor', 'end')
            .attr('fill', '#e0e0e0').style('font-size', '13px').style('font-weight', '400')
            .text(comp.displayName.substring(0, 50));
        // Values
        const ratio = ((comp.ourValue / comp.industryAvg) * 100).toFixed(0);
        svg.append('text').attr('x', Math.max(x(comp.ourValue), x(comp.industryAvg)) + 10)
            .attr('y', y + barH / 2 + 5).attr('fill', col).style('font-size', '12px').style('font-weight', '500')
            .text(`${formatNumber(comp.ourValue)} vs ${formatNumber(comp.industryAvg)} (${ratio}%)`);
    });

    // Legend
    svg.append('rect').attr('x', w - 220).attr('y', -18).attr('width', 14).attr('height', 10)
        .attr('fill', INDUSTRY_LINE_COLOR).attr('opacity', 0.6).attr('rx', 2);
    svg.append('text').attr('x', w - 202).attr('y', -9).attr('fill', '#c0c0c0').style('font-size', '12px').text('Industry Avg');
    svg.append('rect').attr('x', w - 110).attr('y', -18).attr('width', 14).attr('height', 10)
        .attr('fill', '#43e97b').attr('opacity', 0.8).attr('rx', 2);
    svg.append('text').attr('x', w - 92).attr('y', -9).attr('fill', '#c0c0c0').style('font-size', '12px').text('AstraWeave');
}

// ─── HISTOGRAM ───────────────────────────────────────────────────────────────

function renderHistogram(series) {
    const div = document.getElementById('histogram');
    div.innerHTML = '';
    if (series.length === 0) return;

    const vals = series.map(s => ({ name: s.name, value: s.values[s.values.length - 1].value, color: s.color }));
    const margin = { top: 16, right: 20, bottom: 50, left: 70 };
    const w = div.clientWidth - margin.left - margin.right;
    const h = 300 - margin.top - margin.bottom;

    const svg = d3.select(div).append('svg')
        .attr('width', w + margin.left + margin.right).attr('height', h + margin.top + margin.bottom)
        .append('g').attr('transform', `translate(${margin.left},${margin.top})`);

    const histMaxVal = d3.max(vals, d => d.value);
    const histPositive = vals.filter(v => v.value > 0);
    const histMinVal = histPositive.length > 0 ? d3.min(histPositive, d => d.value) : 1;
    const histUseLog = (histMaxVal / Math.max(histMinVal, 1e-10)) > 100;

    let x, bins;
    if (histUseLog) {
        x = d3.scaleSymlog().constant(Math.max(histMinVal * 0.1, 0.1)).domain([0, histMaxVal * 1.05]).range([0, w]);
        // Generate log-spaced thresholds for meaningful bins
        const logMin = Math.log10(Math.max(histMinVal * 0.5, 0.1));
        const logMax = Math.log10(histMaxVal * 1.05);
        const thresholds = d3.range(logMin, logMax, (logMax - logMin) / 20).map(d => Math.pow(10, d));
        bins = d3.bin().thresholds(thresholds).value(d => d.value)(vals);
    } else {
        x = d3.scaleLinear().domain([0, histMaxVal * 1.05]).range([0, w]);
        bins = d3.bin().thresholds(20).value(d => d.value)(vals);
    }
    const y = d3.scaleLinear().range([h, 0]).domain([0, d3.max(bins, d => d.length)]);

    // Tooltip for histogram bars
    const tooltip = d3.select('.tooltip');
    svg.selectAll('rect').data(bins).enter().append('rect')
        .attr('x', d => x(d.x0)).attr('y', d => y(d.length))
        .attr('width', d => Math.max(2, x(d.x1) - x(d.x0) - 2))
        .attr('height', d => h - y(d.length)).attr('fill', '#4facfe').attr('opacity', 0.75)
        .attr('rx', 2)
        .on('mouseenter', function(event, d) {
            d3.select(this).attr('opacity', 1).attr('fill', '#00f2fe');
            const names = d.map(v => v.name).slice(0, 5).join('<br>');
            const more = d.length > 5 ? `<br>+${d.length - 5} more` : '';
            tooltip.style('left', (event.pageX + 12) + 'px').style('top', (event.pageY - 20) + 'px')
                .classed('visible', true)
                .html(`<strong>${d.length} benchmark${d.length > 1 ? 's' : ''}</strong><br>Range: ${formatNumber(d.x0)} – ${formatNumber(d.x1)}<br>${names}${more}`);
        })
        .on('mouseleave', function() {
            d3.select(this).attr('opacity', 0.75).attr('fill', '#4facfe');
            tooltip.classed('visible', false);
        });

    const histXAxisGen = histUseLog
        ? d3.axisBottom(x).ticks(8).tickFormat(d => d === 0 ? '0' : formatNumber(d))
        : d3.axisBottom(x).ticks(8).tickFormat(d => formatNumber(d));
    svg.append('g').attr('class', 'axis').attr('transform', `translate(0,${h})`)
        .call(histXAxisGen)
        .selectAll('text').style('font-size', '12px');
    svg.append('g').attr('class', 'axis').call(d3.axisLeft(y).ticks(6))
        .selectAll('text').style('font-size', '12px');

    // Axis labels
    svg.append('text').attr('x', w / 2).attr('y', h + 42)
        .attr('text-anchor', 'middle').attr('fill', '#b0b0b0').style('font-size', '13px')
        .text(histUseLog ? 'Benchmark Value (log scale)' : 'Benchmark Value');
    svg.append('text').attr('transform', 'rotate(-90)').attr('y', -52).attr('x', -h / 2)
        .attr('text-anchor', 'middle').attr('fill', '#b0b0b0').style('font-size', '13px').text('Count');
}

// ─── SPARKLINES ──────────────────────────────────────────────────────────────

function renderSparklines(series) {
    const container = document.getElementById('sparklines');
    container.innerHTML = '';
    if (series.length === 0) return;
    series.slice(0, 64).forEach((s, idx) => {
        const box = document.createElement('div');
        box.className = 'sparkline-cell';
        box.title = 'Click to enlarge';
        box.addEventListener('click', () => enlargeSparkline(s));

        const label = document.createElement('div');
        label.className = 'spark-label';
        label.textContent = s.name;
        box.appendChild(label);

        // Show latest value
        const latestVal = s.values[s.values.length - 1];
        const valDiv = document.createElement('div');
        valDiv.className = 'spark-value';
        valDiv.textContent = latestVal ? formatDuration(latestVal.value) : '';
        box.appendChild(valDiv);

        // Enlarge hint icon
        const hint = document.createElement('div');
        hint.className = 'spark-enlarge-hint';
        hint.textContent = '🔍';
        box.appendChild(hint);

        const spark = document.createElement('div');
        box.appendChild(spark);

        const sparkH = 72;
        const sparkSvg = d3.select(spark).append('svg').attr('width', '100%').attr('height', sparkH).append('g');
        const w = 240, h = sparkH;

        // Single data point: render a mini bar instead of a line
        if (s.values.length <= 1) {
            const barWidth = w * 0.6;
            sparkSvg.append('rect')
                .attr('x', (w - barWidth) / 2).attr('y', h * 0.2)
                .attr('width', barWidth).attr('height', h * 0.5)
                .attr('fill', s.color).attr('opacity', 0.25).attr('rx', 4);
            sparkSvg.append('rect')
                .attr('x', (w - barWidth) / 2).attr('y', h * 0.2)
                .attr('width', barWidth).attr('height', h * 0.5)
                .attr('fill', 'none').attr('stroke', s.color).attr('stroke-width', 1.5).attr('rx', 4);
            sparkSvg.append('text')
                .attr('x', w / 2).attr('y', h * 0.52)
                .attr('text-anchor', 'middle').attr('fill', s.color)
                .style('font-size', '13px').style('font-weight', '600')
                .text(formatNumber(latestVal ? latestVal.value : 0));
            sparkSvg.append('text')
                .attr('x', w / 2).attr('y', h * 0.85)
                .attr('text-anchor', 'middle').attr('fill', '#888')
                .style('font-size', '9px')
                .text('single snapshot');
        } else {
            // Multiple points: normal sparkline
            const xs = d3.scaleTime().domain(d3.extent(s.values, d => d.timestamp)).range([0, w]);
            const yMin = d3.min(s.values, d => d.value);
            const yMax = d3.max(s.values, d => d.value);
            const ys = d3.scaleLinear().domain([yMin, yMax]).range([h - 6, 6]);
            const line = d3.line().x(d => xs(d.timestamp)).y(d => ys(d.value)).curve(d3.curveBasis);

            // Area fill under curve
            const area = d3.area().x(d => xs(d.timestamp)).y0(h).y1(d => ys(d.value)).curve(d3.curveBasis);
            sparkSvg.append('path').datum(s.values).attr('d', area)
                .attr('fill', s.color).attr('opacity', 0.08);

            sparkSvg.append('path').datum(s.values).attr('d', line)
                .attr('stroke', s.color).attr('stroke-width', 2).attr('fill', 'none');

            // End dot
            if (latestVal) {
                sparkSvg.append('circle')
                    .attr('cx', xs(latestVal.timestamp)).attr('cy', ys(latestVal.value))
                    .attr('r', 3).attr('fill', s.color).attr('opacity', 0.9);
            }
        }

        container.appendChild(box);
    });
}

// ─── BENCHMARK TABLE ─────────────────────────────────────────────────────────

function renderBenchTable() {
    const c = document.getElementById('benchmark-table');
    c.innerHTML = '';
    const groups = d3.group(filteredData, d => d.benchmark_name);
    const rows = [];
    groups.forEach((vals, name) => {
        vals.sort((a, b) => a.timestamp - b.timestamp);
        const latest = vals[vals.length - 1], oldest = vals[0];
        const change = vals.length > 1 ? ((latest.value - oldest.value) / oldest.value) * 100 : 0;
        const sys = detectSystem(latest.crate);
        rows.push({ name, latest, change, ref: findIndustryStandard(name, sys), system: sys });
    });
    rows.sort((a, b) => b.latest.value - a.latest.value);

    const table = document.createElement('table');
    table.style.cssText = 'width:100%;border-collapse:collapse;';
    c.appendChild(table);

    const head = document.createElement('thead');
    const hr = document.createElement('tr');
    ['Benchmark', 'System', 'Latest', 'p50', 'p95', 'CV%', 'Change', 'vs Industry', 'Hardware'].forEach(h => {
        const th = document.createElement('th');
        th.style.cssText = 'text-align:left;padding:6px 10px;color:#a0a0a0;font-size:0.85em;';
        th.textContent = h; hr.appendChild(th);
    });
    head.appendChild(hr); table.appendChild(head);

    const body = document.createElement('tbody');
    rows.forEach(r => {
        const tr = document.createElement('tr');
        tr.style.borderTop = '1px solid rgba(255,255,255,0.03)';

        const td = (text, style) => { const el = document.createElement('td'); el.style.cssText = 'padding:6px 10px;' + (style || ''); el.innerHTML = text; return el; };

        // Compute per-row stats
        const allVals = filteredData.filter(d => d.benchmark_name === r.name).map(d => d.value);
        const p50 = percentile(allVals, 50);
        const p95 = percentile(allVals, 95);
        const cv = coefficientOfVariation(allVals);

        tr.appendChild(td(r.latest.display_name || r.name));
        tr.appendChild(td(`<span style="color:${COLOR_SCHEME[r.system]||'#a0a0a0'};font-size:0.8em;">${r.system}</span>`));
        tr.appendChild(td(`${formatNumber(r.latest.value)} ${r.latest.unit}`));
        tr.appendChild(td(`${formatNumber(p50)}`, 'color:#a0a0a0;font-size:0.85em;'));
        tr.appendChild(td(`${formatNumber(p95)}`, 'color:#a0a0a0;font-size:0.85em;'));
        const cvColor = cv < 5 ? '#43e97b' : cv < 15 ? '#feca57' : '#ff6b6b';
        tr.appendChild(td(`<span style="color:${cvColor}">${cv.toFixed(1)}%</span>`, 'font-size:0.85em;'));
        tr.appendChild(td(`${r.change > 0 ? '+' : ''}${r.change.toFixed(1)}%`, `color:${r.change > 0 ? '#ff6b6b' : '#43e97b'}`));

        let indTxt = '—';
        if (r.ref && currentFilters.showIndustry) {
            const ratio = ((r.latest.value / r.ref.average) * 100).toFixed(0);
            const ic = r.latest.value <= r.ref.average ? '#43e97b' : '#ff6b6b';
            indTxt = `<span style="color:${ic}">${ratio}%</span>`;
        }
        tr.appendChild(td(indTxt));
        tr.appendChild(td(r.latest.hardware_id === BASELINE_HARDWARE_ID ? '🏠 Baseline' : '🖥️ User', 'font-size:0.8em;color:#a0a0a0;'));

        body.appendChild(tr);
    });
    table.appendChild(body);
}

// ─── LEGEND ──────────────────────────────────────────────────────────────────

function renderLegend(series, industryLines = []) {
    const div = document.getElementById('legend');
    div.innerHTML = '';
    const LEGEND_SHOW = 20;
    const visible = series.slice(0, LEGEND_SHOW);
    visible.forEach(s => {
        const item = document.createElement('div'); item.className = 'legend-item';
        const color = document.createElement('div'); color.className = 'legend-color';
        color.style.backgroundColor = s.color;
        if (!s.isBaseline) color.style.borderTop = '2px dashed ' + s.color;
        const label = document.createElement('span'); label.textContent = s.name;
        item.appendChild(color); item.appendChild(label); div.appendChild(item);
    });
    if (series.length > LEGEND_SHOW) {
        const more = document.createElement('div');
        more.className = 'legend-item';
        more.style.cssText = 'color:#a0a0a0;font-style:italic;font-size:0.85em;cursor:pointer;';
        more.textContent = `+ ${series.length - LEGEND_SHOW} more (use filters to narrow)`;
        more.addEventListener('click', () => {
            // Toggle showing all
            if (more.dataset.expanded === 'true') {
                more.dataset.expanded = 'false';
                more.textContent = `+ ${series.length - LEGEND_SHOW} more (click to show)`;
                div.querySelectorAll('.legend-extra').forEach(el => el.remove());
            } else {
                more.dataset.expanded = 'true';
                more.textContent = '− collapse';
                series.slice(LEGEND_SHOW).forEach(s => {
                    const item = document.createElement('div'); item.className = 'legend-item legend-extra';
                    const color = document.createElement('div'); color.className = 'legend-color';
                    color.style.backgroundColor = s.color;
                    const label = document.createElement('span'); label.textContent = s.name;
                    item.appendChild(color); item.appendChild(label); div.appendChild(item);
                });
            }
        });
        div.appendChild(more);
    }
    if (industryLines.length > 0 && currentFilters.showIndustry) {
        const item = document.createElement('div'); item.className = 'legend-item';
        const color = document.createElement('div'); color.className = 'legend-color';
        color.style.backgroundColor = INDUSTRY_LINE_COLOR;
        color.style.borderTop = '2px dashed ' + INDUSTRY_LINE_COLOR;
        const label = document.createElement('span'); label.textContent = 'Industry Standard';
        label.style.color = INDUSTRY_LINE_COLOR;
        item.appendChild(color); item.appendChild(label); div.appendChild(item);
    }
}

// ─── REGRESSION ALERTS ───────────────────────────────────────────────────────

function renderRegressionAlerts() {
    const c = document.getElementById('regression-alerts');
    if (!c) return;
    c.innerHTML = '';
    const groups = {};
    filteredData.forEach(d => { (groups[d.benchmark_name] = groups[d.benchmark_name] || []).push(d); });

    const regs = [];
    for (const [name, entries] of Object.entries(groups)) {
        entries.sort((a, b) => a.timestamp - b.timestamp);
        if (entries.length < 2) continue;
        const change = ((entries[entries.length - 1].value - entries[0].value) / entries[0].value) * 100;
        if (change > 10) regs.push({ name, baseline: entries[0].value, current: entries[entries.length - 1].value, change, unit: entries[entries.length - 1].unit });
    }

    if (regs.length === 0) {
        c.innerHTML = `<div class="alert-success">✅ <strong>No Performance Regressions Detected</strong>
            <p style="margin-top:8px;font-size:0.9em;color:#a0a0a0;">All ${Object.keys(groups).length} benchmarks within ±10%</p></div>`;
        return;
    }
    regs.sort((a, b) => b.change - a.change);
    c.innerHTML = `<div class="alert-warning">⚠️ <strong>${regs.length} Regression${regs.length > 1 ? 's' : ''}</strong>
        <div style="margin-top:12px;max-height:200px;overflow-y:auto;">
            ${regs.slice(0, 10).map(r => `<div style="padding:8px;border-left:3px solid #ff6b6b;margin:6px 0;background:rgba(255,107,107,0.1);border-radius:4px;">
                <div style="font-weight:600;color:#ff6b6b;">${r.name}</div>
                <div style="font-size:0.85em;color:#a0a0a0;margin-top:4px;">${formatNumber(r.baseline)} → ${formatNumber(r.current)} ${r.unit}
                    <span style="color:#ff6b6b;font-weight:600;">(+${r.change.toFixed(1)}%)</span></div>
            </div>`).join('')}
        </div></div>`;
}

// ─── PRODUCTION HEALTH ───────────────────────────────────────────────────────

function renderProductionHealthSummary() {
    const c = document.getElementById('production-health');
    if (!c) return;

    const uniqueB = new Set(filteredData.map(d => d.benchmark_name)).size;
    const uniqueC = new Set(filteredData.map(d => d.crate)).size;
    const uniqueH = new Set(filteredData.map(d => d.hardware_id)).size;
    const dateRange = filteredData.length > 0
        ? Math.ceil((Math.max(...filteredData.map(d => d.timestamp)) - Math.min(...filteredData.map(d => d.timestamp))) / 86400000) : 0;

    const groups = {};
    filteredData.forEach(d => { (groups[d.benchmark_name] = groups[d.benchmark_name] || []).push(d); });
    let regCount = 0;
    for (const entries of Object.values(groups)) {
        entries.sort((a, b) => a.timestamp - b.timestamp);
        if (entries.length >= 2 && ((entries[entries.length - 1].value - entries[0].value) / entries[0].value) * 100 > 10) regCount++;
    }
    const total = Object.keys(groups).length;
    const pct = total > 0 ? (regCount / total) * 100 : 0;
    let grade = 'A+', gc = '#00f2fe';
    if (pct > 20) { grade = 'C'; gc = '#ff6b6b'; }
    else if (pct > 10) { grade = 'B'; gc = '#feca57'; }
    else if (pct > 5) { grade = 'A'; gc = '#43e97b'; }

    let indCount = 0;
    for (const name of Object.keys(groups)) {
        const sys = detectSystem(filteredData.find(d => d.benchmark_name === name)?.crate || '');
        if (findIndustryStandard(name, sys)) indCount++;
    }

    const sysCounts = {};
    filteredData.forEach(d => { const s = detectSystem(d.crate); sysCounts[s] = (sysCounts[s] || 0) + 1; });

    c.innerHTML = `
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:16px;">
            ${[
                [grade, gc, 'Health Grade'],
                [uniqueB, '#4facfe', 'Benchmarks'],
                [uniqueC, '#4facfe', 'Crates'],
                [uniqueH, '#4facfe', 'Hardware Profiles'],
                [`${dateRange}d`, '#4facfe', 'Data Range'],
                [indCount, INDUSTRY_LINE_COLOR, 'Industry Comparisons']
            ].map(([v, color, label]) => `
                <div style="text-align:center;padding:16px;background:rgba(255,255,255,0.03);border-radius:8px;">
                    <div style="font-size:${String(v).length > 3 ? '1.5' : '2'}em;font-weight:bold;color:${color};">${v}</div>
                    <div style="color:#a0a0a0;font-size:0.85em;margin-top:4px;">${label}</div>
                </div>`).join('')}
        </div>
        <div style="margin-top:16px;display:flex;flex-wrap:wrap;gap:8px;justify-content:center;">
            ${Object.entries(sysCounts).map(([s, cnt]) =>
                `<span style="padding:4px 12px;background:${COLOR_SCHEME[s]||'#666'}22;color:${COLOR_SCHEME[s]||'#666'};border-radius:16px;font-size:0.8em;">${s}: ${cnt}</span>`
            ).join('')}
        </div>`;
}

// ─── UTILITIES ───────────────────────────────────────────────────────────────

function updateGeneratedTime() {
    document.getElementById('generated-time').textContent = new Date().toLocaleString();
    // Check static pre-rendered images and hide entire section if none exist
    const imgIds = ['top-series', 'distribution', 'heatmap'];
    let loadedCount = 0;
    let checkedCount = 0;
    imgIds.forEach(id => {
        const img = document.getElementById('static-' + id);
        if (!img) { checkedCount++; return; }
        fetch(img.src, { method: 'HEAD' }).then(r => {
            img.style.display = r.ok ? 'inline-block' : 'none';
            if (r.ok) loadedCount++;
            checkedCount++;
            // Once all checked, hide the entire static section if nothing loaded
            if (checkedCount >= imgIds.length) {
                const staticWrapper = img.closest('div')?.parentElement;
                const staticLabel = staticWrapper?.querySelector('[style*="Static Pre-rendered"]') ||
                    Array.from(staticWrapper?.querySelectorAll('div') || []).find(el => el.textContent.includes('Static Pre-rendered'));
                if (loadedCount === 0) {
                    // Hide the label and the image container
                    if (staticLabel) staticLabel.style.display = 'none';
                    imgIds.forEach(sid => { const im = document.getElementById('static-' + sid); if (im) im.parentElement.style.display = 'none'; });
                }
            }
        }).catch(() => {
            img.style.display = 'none';
            checkedCount++;
            if (checkedCount >= imgIds.length && loadedCount === 0) {
                const staticWrapper = img.closest('div')?.parentElement;
                const staticLabel = Array.from(staticWrapper?.querySelectorAll('div') || []).find(el => el.textContent.includes('Static Pre-rendered'));
                if (staticLabel) staticLabel.style.display = 'none';
                imgIds.forEach(sid => { const im = document.getElementById('static-' + sid); if (im) im.parentElement.style.display = 'none'; });
            }
        });
    });
}

function showError(msg) {
    document.getElementById('stats-grid').innerHTML = `
        <div class="error"><h2>Error Loading Dashboard</h2>
            <p>${msg.replace(/\n/g, '<br>')}</p>
            <h3 style="margin-top:20px;">Generate Benchmark Data:</h3>
            <div style="text-align:left;max-width:600px;margin:20px auto;">
                <p><strong>One-Command (Recommended):</strong></p>
                <pre>.\\scripts\\run_benchmark_dashboard.ps1</pre>
                <p style="margin-top:10px;"><strong>Manual:</strong></p>
                <pre>cargo bench\n.\\scripts\\export_benchmark_jsonl.ps1</pre>
            </div>
        </div>`;
}

function showLoadingSkeleton() {
    const grid = document.getElementById('stats-grid');
    if (grid) {
        grid.innerHTML = Array.from({ length: 6 }, () =>
            `<div class="stat-card skeleton" style="height:120px;border-left-color:transparent;"></div>`
        ).join('');
    }
    ['production-health', 'regression-alerts', 'chart', 'industry-comparison', 'benchmark-table'].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.innerHTML = '<div class="skeleton" style="height:80px;margin:8px 0;"></div>';
    });
}

// ─── LIGHTBOX / ENLARGE FUNCTIONS ────────────────────────────────────────────

function openLightbox(title, contentFn) {
    const overlay = document.getElementById('chart-lightbox');
    const titleEl = document.getElementById('lb-title');
    const body = document.getElementById('lb-body');
    titleEl.textContent = title;
    body.innerHTML = '';
    contentFn(body);
    overlay.classList.add('active');
    // Close on Escape
    document.addEventListener('keydown', _lightboxEscHandler);
}

function closeLightbox(event) {
    if (event && event.target && !event.target.closest('.chart-lightbox-content') && event.target !== document.getElementById('chart-lightbox')) {
        // clicked outside but not on overlay itself — ignore
    }
    const overlay = document.getElementById('chart-lightbox');
    overlay.classList.remove('active');
    document.removeEventListener('keydown', _lightboxEscHandler);
}

function _lightboxEscHandler(e) {
    if (e.key === 'Escape') closeLightbox();
}

function enlargeImage(imgEl) {
    openLightbox(imgEl.alt || 'Graph', (body) => {
        const clone = imgEl.cloneNode(true);
        clone.style.cssText = 'max-width:100%;max-height:75vh;height:auto;border-radius:8px;';
        clone.className = '';
        body.appendChild(clone);
    });
}

function enlargeSparkline(seriesData) {
    openLightbox(seriesData.name, (body) => {
        const chartDiv = document.createElement('div');
        chartDiv.style.cssText = 'width:min(860px, 85vw);height:380px;';
        body.appendChild(chartDiv);

        const margin = { top: 20, right: 30, bottom: 50, left: 80 };
        const w = Math.min(860, window.innerWidth * 0.82) - margin.left - margin.right;
        const h = 380 - margin.top - margin.bottom;

        const svg = d3.select(chartDiv).append('svg')
            .attr('width', w + margin.left + margin.right)
            .attr('height', h + margin.top + margin.bottom)
            .append('g').attr('transform', `translate(${margin.left},${margin.top})`);

        const xs = d3.scaleTime().domain(d3.extent(seriesData.values, d => d.timestamp)).range([0, w]);
        const yMin = d3.min(seriesData.values, d => d.value) * 0.95;
        const yMax = d3.max(seriesData.values, d => d.value) * 1.05;
        const ys = d3.scaleLinear().domain([yMin, yMax]).range([h, 0]);

        // Grid
        svg.append('g').attr('class', 'grid').attr('transform', `translate(0,${h})`)
            .call(d3.axisBottom(xs).tickSize(-h).tickFormat(''));
        svg.append('g').attr('class', 'grid').call(d3.axisLeft(ys).tickSize(-w).tickFormat(''));

        // Axes
        svg.append('g').attr('class', 'axis').attr('transform', `translate(0,${h})`)
            .call(d3.axisBottom(xs).ticks(8).tickFormat(d3.timeFormat('%b %d')))
            .selectAll('text').style('font-size', '13px').attr('fill', '#c0c0c0');
        svg.append('g').attr('class', 'axis').call(d3.axisLeft(ys).ticks(8).tickFormat(d => formatNumber(d)))
            .selectAll('text').style('font-size', '13px').attr('fill', '#c0c0c0');

        // Area
        const area = d3.area().x(d => xs(d.timestamp)).y0(h).y1(d => ys(d.value)).curve(d3.curveMonotoneX);
        svg.append('path').datum(seriesData.values).attr('d', area)
            .attr('fill', seriesData.color).attr('opacity', 0.12);

        // Line
        const line = d3.line().x(d => xs(d.timestamp)).y(d => ys(d.value)).curve(d3.curveMonotoneX);
        svg.append('path').datum(seriesData.values).attr('d', line)
            .attr('stroke', seriesData.color).attr('stroke-width', 2.5).attr('fill', 'none');

        // Data points
        svg.selectAll('circle').data(seriesData.values).enter().append('circle')
            .attr('cx', d => xs(d.timestamp)).attr('cy', d => ys(d.value))
            .attr('r', 4).attr('fill', seriesData.color).attr('opacity', 0.85)
            .attr('stroke', '#fff').attr('stroke-width', 1);

        // Tooltip
        const tooltip = d3.select('.tooltip');
        svg.append('rect').attr('width', w).attr('height', h)
            .attr('fill', 'none').attr('pointer-events', 'all')
            .on('mousemove', function(event) {
                const [mx] = d3.pointer(event);
                const x0 = xs.invert(mx);
                let closest = null, closestD = Infinity;
                seriesData.values.forEach(d => { const dist = Math.abs(d.timestamp - x0); if (dist < closestD) { closestD = dist; closest = d; } });
                if (closest) {
                    tooltip.style('left', (event.pageX + 15) + 'px').style('top', (event.pageY - 28) + 'px')
                        .classed('visible', true)
                        .html(`<strong>${closest.display_name || closest.benchmark_name}</strong><br>Value: ${formatDuration(closest.value)}<br>Date: ${d3.timeFormat('%Y-%m-%d %H:%M')(closest.timestamp)}`);                }
            })
            .on('mouseleave', () => tooltip.classed('visible', false));

        // Axis labels
        svg.append('text').attr('x', w / 2).attr('y', h + 42)
            .attr('text-anchor', 'middle').attr('fill', '#b0b0b0').style('font-size', '13px').text('Date');
        svg.append('text').attr('transform', 'rotate(-90)').attr('y', -60).attr('x', -h / 2)
            .attr('text-anchor', 'middle').attr('fill', '#b0b0b0').style('font-size', '13px').text('Value');

        // Stats summary below chart
        const stats = document.createElement('div');
        stats.style.cssText = 'margin-top:16px;display:flex;gap:24px;flex-wrap:wrap;color:#c0c0c0;font-size:13px;';
        const allV = seriesData.values.map(d => d.value);
        const latest = allV[allV.length - 1];
        const min = Math.min(...allV);
        const max = Math.max(...allV);
        const avg = allV.reduce((a, b) => a + b, 0) / allV.length;
        const p50 = percentile(allV, 50);
        const cv = coefficientOfVariation(allV);
        stats.innerHTML = [
            `Latest: <strong style="color:#4facfe">${formatNumber(latest)}</strong>`,
            `Min: <strong>${formatNumber(min)}</strong>`,
            `Max: <strong>${formatNumber(max)}</strong>`,
            `Avg: <strong>${formatNumber(avg)}</strong>`,
            `p50: <strong>${formatNumber(p50)}</strong>`,
            `CV: <strong style="color:${cv < 5 ? '#43e97b' : cv < 15 ? '#feca57' : '#ff6b6b'}">${cv.toFixed(1)}%</strong>`,
            `Samples: <strong>${allV.length}</strong>`,
        ].map(s => `<span>${s}</span>`).join('');
        body.appendChild(stats);
    });
}

// ─── EXPORT FUNCTIONS ───────────────────────────────────────────────────────

function exportCSV() {
    const data = filteredData.length > 0 ? filteredData : benchmarkData;
    const headers = ['benchmark_name','display_name','crate','group','value','stddev','unit','timestamp','git_branch','git_sha','hardware_id','hardware_label','system','p50','p95','cv_pct'];
    const groups = {};
    data.forEach(d => { (groups[d.benchmark_name] = groups[d.benchmark_name] || []).push(d); });
    const rows = data.map(d => {
        const grpVals = (groups[d.benchmark_name] || []).map(e => e.value);
        const sys = detectSystem(d.crate);
        return headers.map(h => {
            if (h === 'system') return sys;
            if (h === 'p50') return percentile(grpVals, 50).toFixed(2);
            if (h === 'p95') return percentile(grpVals, 95).toFixed(2);
            if (h === 'cv_pct') return coefficientOfVariation(grpVals).toFixed(2);
            if (h === 'timestamp') return d.timestamp instanceof Date ? d.timestamp.toISOString() : d.timestamp;
            const v = d[h];
            return v !== undefined && v !== null ? String(v).replace(/,/g, ';') : '';
        }).join(',');
    });
    const csv = [headers.join(','), ...rows].join('\n');
    downloadFile(csv, 'astraweave-benchmarks.csv', 'text/csv');
    showToast(`Exported ${data.length} records as CSV`);
}

function exportJSON() {
    const data = filteredData.length > 0 ? filteredData : benchmarkData;
    const exported = data.map(d => ({
        ...d,
        timestamp: d.timestamp instanceof Date ? d.timestamp.toISOString() : d.timestamp,
        system: detectSystem(d.crate)
    }));
    const json = JSON.stringify(exported, null, 2);
    downloadFile(json, 'astraweave-benchmarks.json', 'application/json');
    showToast(`Exported ${data.length} records as JSON`);
}

function shareSnapshot() {
    persistFiltersToURL();
    const url = location.href;
    navigator.clipboard.writeText(url).then(() => {
        showToast('\ud83d\udd17 Dashboard URL copied to clipboard!');
    }).catch(() => {
        // Fallback for non-HTTPS contexts
        prompt('Copy this URL:', url);
    });
}

function downloadFile(content, filename, mime) {
    const blob = new Blob([content], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = filename;
    document.body.appendChild(a); a.click();
    setTimeout(() => { document.body.removeChild(a); URL.revokeObjectURL(url); }, 100);
}

// ─── INIT ────────────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
    console.log('AstraWeave Benchmark Dashboard v7.0');
    loadBenchmarkData();
});

// Resize handler — debounced re-render on window resize
let resizeTimer;
window.addEventListener('resize', () => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
        if (benchmarkData.length > 0) renderDashboard();
    }, 250);
});
