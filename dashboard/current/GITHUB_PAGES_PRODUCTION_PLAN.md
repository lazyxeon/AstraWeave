# AstraWeave GitHub Pages Production-Grade Transformation Plan

**Date**: 2026-01-02  
**Status**: Strategic Plan  
**Authors**: Multi-Agent Analysis (Explorer, Auditor, Maintainer, ToT-Reasoner)

---

## Executive Summary

AstraWeave has a **documentation foundation** with 1,078+ files and 2M+ words of content, but the GitHub Pages deployment requires significant infrastructure improvements to achieve production-grade quality. This plan addresses:

- **CRITICAL**: Deployment conflict between two workflows that destroys content
- **Content Gaps**: 40-50% of mdBook pages are stubs
- **Missing Features**: 12/12 production-grade features not implemented
- **Infrastructure**: Zero mdBook plugins installed (vanilla configuration)

**Current Grade**: D+ (Functional but fragile)  
**Target Grade**: A (Production-ready, best-in-class)  
**Estimated Total Effort**: 9-17 hours for core fixes, 12 weeks for full transformation

---

## Current State Analysis

### What We Have

| Component | Status | Notes |
|-----------|--------|-------|
| **mdBook Framework** | Working | docs/book.toml configured |
| **54 Source Pages** | docs/src/ | SUMMARY.md navigation |
| **Benchmark Dashboard** | Working | tools/benchmark-dashboard/ |
| **Cargo Doc Integration** | Partial | 5 core crates documented |
| **GitHub Actions** | 2 Workflows | docs.yml + benchmark-dashboard.yml |
| **gh-pages Branch** | Exists | Active deployment target |

### Critical Issues Identified

| ID | Issue | Severity | Impact |
|----|-------|----------|--------|
| **C-001** | Two different deployment actions CONFLICT | CRITICAL | Last writer wins; docs or dashboard destroyed |
| **C-002** | No `.nojekyll` file | HIGH | GitHub may break asset paths |
| **C-003** | 5 broken external links (Veilweaver) | HIGH | 404 errors in navigation |
| **C-004** | 20+ stub pages (<50 bytes) | HIGH | Poor user experience |
| **C-005** | No mdBook plugins installed | MEDIUM | Missing diagrams, callouts, link checking |

### Content Completeness

```
Current Content Status:

Complete Pages (>100 lines):    ~10 pages  (18%)
Partial Pages (50-100 lines):   ~15 pages  (28%)  
Stub Pages (<50 bytes):         ~29 pages  (54%)

Highest Quality:
- architecture/overview.md (386 lines)
- architecture/ai-native.md (685 lines)
- getting-started/installation.md (306 lines)
- getting-started/quick-start.md (114 lines)
- examples/index.md (317 lines)

Critical Stubs (All Core Systems):
- core-systems/audio.md (9 bytes)
- core-systems/physics.md (11 bytes)
- core-systems/rendering.md (13 bytes)
- core-systems/navigation.md (14 bytes)
- core-systems/networking.md (14 bytes)
- core-systems/input.md (16 bytes)
- All dev/*.md pages (stubs)
```

---

## Phase 0: Critical Infrastructure Fixes (1-2 Hours)

**Priority**: MUST DO IMMEDIATELY before any new deployments

### Task 0.1: Unify GitHub Pages Deployment Actions

**Problem**: `docs.yml` uses `JamesIves/github-pages-deploy-action@v4` while `benchmark-dashboard.yml` uses `peaceiris/actions-gh-pages@v4` with `force_orphan: true`, causing mutual destruction.

**Solution**: Standardize on `peaceiris/actions-gh-pages@v4` with `keep_files: true` and use subdirectory deployment.

**Changes Required**:

1. **Update `.github/workflows/docs.yml`**:
```yaml
# Replace line 58-63:
- name: Deploy to GitHub Pages
  if: github.event_name == 'push' && github.ref == 'refs/heads/main'
  uses: peaceiris/actions-gh-pages@v4
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./docs/book
    publish_branch: gh-pages
    keep_files: true
    commit_message: 'Deploy mdBook documentation'
```

2. **Update `.github/workflows/benchmark-dashboard.yml`**:
```yaml
# Replace line 142-149 (remove force_orphan, add destination_dir):
- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v4
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./gh-pages
    publish_branch: gh-pages
    keep_files: true
    destination_dir: benchmarks
    commit_message: 'Update benchmark dashboard: ${{ github.sha }}'
```

**Result**:
- mdBook docs at: `https://lazyxeon.github.io/AstraWeave/`
- Benchmarks at: `https://lazyxeon.github.io/AstraWeave/benchmarks/`

**Effort**: 15 minutes

---

### Task 0.2: Add `.nojekyll` File

**Problem**: GitHub Pages may process files through Jekyll, breaking certain assets.

**Solution**: Add to `docs.yml` workflow:

```yaml
- name: Build mdBook documentation
  run: |
    cd docs
    mdbook build
    mkdir -p book/api
    cp -r ../target/doc/* book/api/
    echo '<meta http-equiv="refresh" content="0; url=astraweave_core/index.html">' > book/api/index.html
    touch book/.nojekyll  # <-- ADD THIS LINE
```

**Effort**: 2 minutes

---

### Task 0.3: Fix Broken SUMMARY.md Links

**Problem**: Lines 55-58 reference non-existent `../../Games-VEILWEAVER/` directory.

**Solution**: Comment out or replace with placeholder:

```markdown
# Reference Implementation
# Note: Veilweaver is developed in a separate repository
- [Veilweaver Overview](./veilweaver-placeholder.md)
```

Create `docs/src/veilweaver-placeholder.md`:
```markdown
# Veilweaver: Reference Implementation

> **Coming Soon**: Veilweaver documentation is being migrated to this documentation site.

For now, see the [GitHub repository](https://github.com/lazyxeon/Games-VEILWEAVER) for the latest Veilweaver information.
```

Also fix line 85 (broken roadmap link):
```markdown
- [Roadmap](./resources/roadmap.md)  # Use actual local path
```

**Effort**: 15 minutes

---

### Task 0.4: Validate Unified Deployment

**Action**: After implementing Tasks 0.1-0.3:
1. Push changes to main branch
2. Wait for both workflows to complete
3. Verify both URLs accessible:
   - https://lazyxeon.github.io/AstraWeave/
   - https://lazyxeon.github.io/AstraWeave/benchmarks/

**Effort**: 30 minutes (including wait time)

---

## Phase 1: Production Essentials (2-4 Hours)

**Priority**: Required for professional documentation site

### Task 1.1: Install Essential mdBook Plugins

**Plugins to Install**:
| Plugin | Purpose | Priority |
|--------|---------|----------|
| `mdbook-mermaid` | Architecture diagrams | P0 |
| `mdbook-admonish` | Warning/note callouts | P0 |
| `mdbook-linkcheck` | Validate internal/external links | P0 |

**Update `docs/book.toml`**:
```toml
[preprocessor.mermaid]
command = "mdbook-mermaid"

[preprocessor.admonish]
command = "mdbook-admonish"
assets_version = "3.0.2"

[output.linkcheck]
follow-web-links = true
traverse-parent-directories = false
exclude = ["localhost", "127.0.0.1"]
```

**Update `.github/workflows/docs.yml`**:
```yaml
- name: Install mdBook and plugins
  run: |
    cargo install mdbook mdbook-mermaid mdbook-linkcheck mdbook-admonish
    mdbook-admonish install docs  # Initialize admonish CSS
```

**Effort**: 1 hour

---

### Task 1.2: Create Custom 404 Page

**Create `docs/src/404.md`**:
```markdown
# Page Not Found

The page you're looking for doesn't exist or has been moved.

## Quick Links
- [Home](./README.md)
- [Getting Started](./getting-started/quick-start.md)
- [API Documentation](./api/index.md)
- [Examples](./examples/index.md)

## Need Help?
- [FAQ](./resources/faq.md)
- [Troubleshooting](./resources/troubleshooting.md)
- [Report a broken link](https://github.com/lazyxeon/AstraWeave/issues/new?labels=documentation)
```

**Update `docs/book.toml`**:
```toml
[output.html]
input-404 = "404.md"
```

**Effort**: 20 minutes

---

### Task 1.3: Create Functional API Documentation Landing Page

**Rewrite `docs/src/api/index.md`**:
```markdown
# API Documentation

Browse the generated Rust API documentation for AstraWeave crates.

## Core Engine
- [astraweave_core](astraweave_core/index.html) - Core ECS, simulation, and world systems
- [astraweave_ecs](astraweave_ecs/index.html) - Entity Component System implementation

## AI & Behavior
- [astraweave_ai](astraweave_ai/index.html) - AI orchestration and planning
- [astraweave_behavior](astraweave_behavior/index.html) - Behavior trees and GOAP
- [astraweave_llm](astraweave_llm/index.html) - LLM integration layer

## Rendering & Graphics
- [astraweave_render](astraweave_render/index.html) - wgpu-based renderer
- [astraweave_materials](astraweave_materials/index.html) - PBR material system

## Physics & Navigation
- [astraweave_physics](astraweave_physics/index.html) - Rapier3D integration
- [astraweave_nav](astraweave_nav/index.html) - Navmesh and pathfinding

## Gameplay Systems
- [astraweave_gameplay](astraweave_gameplay/index.html) - Combat, crafting, dialogue
- [astraweave_terrain](astraweave_terrain/index.html) - Voxel terrain generation

> **Note**: API docs are auto-generated from source code on every push to main.
```

**Effort**: 30 minutes

---

### Task 1.4: Add Copy-to-Clipboard for Code Blocks

**Create `docs/theme/clipboard.js`**:
```javascript
(function() {
    const blocks = document.querySelectorAll('pre code');
    blocks.forEach(function(block) {
        const button = document.createElement('button');
        button.className = 'copy-button';
        button.textContent = 'Copy';
        button.addEventListener('click', function() {
            navigator.clipboard.writeText(block.textContent);
            button.textContent = 'Copied!';
            setTimeout(() => button.textContent = 'Copy', 2000);
        });
        block.parentNode.insertBefore(button, block);
    });
})();
```

**Create `docs/theme/clipboard.css`**:
```css
.copy-button {
    position: absolute;
    right: 5px;
    top: 5px;
    padding: 4px 8px;
    font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.2s;
}
pre:hover .copy-button { opacity: 1; }
pre { position: relative; }
```

**Update `docs/book.toml`**:
```toml
[output.html]
additional-js = ["theme/clipboard.js"]
additional-css = ["theme/clipboard.css"]
```

**Effort**: 30 minutes

---

### Task 1.5: Link Benchmark Dashboard from Main Docs

**Add to `docs/src/SUMMARY.md`** under Resources:
```markdown
# Resources

- [FAQ](./resources/faq.md)
- [Performance Tips](./resources/performance.md)
- [Best Practices](./resources/best-practices.md)
- [Common Patterns](./resources/patterns.md)
- [Troubleshooting](./resources/troubleshooting.md)
- [Community](./resources/community.md)
- [Roadmap](./resources/roadmap.md)
- [Benchmark Dashboard](../benchmarks/index.html)  # <-- ADD THIS
```

**Effort**: 5 minutes

---

## Phase 2: Content Expansion (1-2 Weeks)

**Priority**: Fill critical content gaps

### Task 2.1: Populate Core Systems Stubs

**Target**: Each core system page should have 100-200 lines with:
- System overview
- Key types/traits
- Basic usage example
- Link to API docs

**Files to expand**:
| File | Current Size | Target Size | Priority |
|------|-------------|-------------|----------|
| `core-systems/physics.md` | 11 bytes | 200+ lines | P0 |
| `core-systems/rendering.md` | 13 bytes | 300+ lines | P0 |
| `core-systems/audio.md` | 9 bytes | 150+ lines | P1 |
| `core-systems/navigation.md` | 14 bytes | 200+ lines | P1 |
| `core-systems/networking.md` | 14 bytes | 200+ lines | P1 |
| `core-systems/input.md` | 16 bytes | 150+ lines | P1 |
| `core-systems/ai/index.md` | 2 lines | 150+ lines | P0 |

**Effort**: 2-3 days

---

### Task 2.2: Complete Development Guides

**Files to expand**:
| File | Priority | Content Focus |
|------|----------|---------------|
| `dev/contributing.md` | P0 | PR process, code review, style |
| `dev/building.md` | P0 | Windows/Linux/Mac build instructions |
| `dev/testing.md` | P1 | Unit tests, integration tests, benchmarks |
| `dev/code-style.md` | P1 | Rust conventions, linting, formatting |
| `dev/new-features.md` | P2 | Feature proposal process |
| `dev/performance.md` | P2 | Profiling, optimization techniques |

**Effort**: 1-2 days

---

### Task 2.3: Complete Getting Started Stubs

**Files to expand**:
| File | Priority |
|------|----------|
| `getting-started/first-companion.md` | P0 |
| `getting-started/requirements.md` | P0 |

**Effort**: 4 hours

---

### Task 2.4: Create Reference Glossary

**Create `docs/src/reference/glossary.md`**:

```markdown
# Glossary

## A
**Arbiter** - The AI system responsible for validating tool calls and ensuring agents can only perform sanctioned actions within the game rules.

**Asset Pipeline** - The content processing workflow that converts raw assets (textures, models, audio) into engine-optimized formats.

## B
**Behavior Tree** - A hierarchical AI decision-making structure used for complex agent behaviors.

## E
**ECS (Entity Component System)** - Data-oriented architecture pattern where entities are IDs, components are data, and systems are logic.

## G
**GOAP (Goal-Oriented Action Planning)** - AI planning algorithm that finds optimal action sequences to achieve goals.

... (continue for all key terms)
```

**Effort**: 4 hours

---

## Phase 3: Professional Polish (1-2 Weeks)

**Priority**: Production-grade enhancements

### Task 3.1: SEO Optimization

**Update `docs/book.toml`**:
```toml
[output.html]
description = "Comprehensive documentation for AstraWeave, an AI-native game engine built in Rust with deterministic ECS architecture."

[output.html.search]
enable = true
limit-results = 30
use-boolean-and = true
boost-title = 2
boost-hierarchy = 1
boost-paragraph = 1
expand = true
heading-split-level = 3
copy-js = true
```

**Effort**: 30 minutes

---

### Task 3.2: Add Sitemap Generation

**Install**: `cargo install mdbook-sitemap`

**Update `docs/book.toml`**:
```toml
[output.sitemap]
hostname = "https://lazyxeon.github.io/AstraWeave"
```

**Create `docs/src/robots.txt`**:
```
User-agent: *
Allow: /
Sitemap: https://lazyxeon.github.io/AstraWeave/sitemap.xml
```

**Effort**: 30 minutes

---

### Task 3.3: Analytics Integration (Optional)

**Add privacy-respecting analytics** (Plausible or Fathom):

**Update `docs/book.toml`**:
```toml
[output.html]
additional-js = ["theme/clipboard.js", "https://plausible.io/js/script.js"]
```

Or for self-hosted analytics, use Umami.

**Effort**: 30 minutes

---

### Task 3.4: Add Open Graph Tags

**Create `docs/theme/head.hbs`** (custom template):
```html
<meta property="og:title" content="AstraWeave Documentation">
<meta property="og:description" content="An AI-native game engine built in Rust">
<meta property="og:image" content="https://lazyxeon.github.io/AstraWeave/assets/og-image.png">
<meta property="og:url" content="https://lazyxeon.github.io/AstraWeave/">
<meta name="twitter:card" content="summary_large_image">
```

**Effort**: 1 hour

---

### Task 3.5: Automated Link Checking in CI

**Add to `.github/workflows/docs.yml`**:
```yaml
- name: Check for broken links
  run: |
    curl -sSL https://github.com/lycheeverse/lychee/releases/download/v0.15.1/lychee-v0.15.1-x86_64-unknown-linux-gnu.tar.gz | tar -xz
    ./lychee docs/book/**/*.html --exclude localhost --format markdown > link-report.md
  continue-on-error: true

- name: Upload link report
  uses: actions/upload-artifact@v4
  if: always()
  with:
    name: link-report
    path: link-report.md
```

**Effort**: 30 minutes

---

### Task 3.6: Version Selector (Pre-1.0 Preparation)

**Multi-version hosting strategy**:
```
gh-pages/
├── latest/           # Current development
├── v0.1.0/          # Tagged release
├── v0.2.0/          # Tagged release
└── benchmarks/      # Performance dashboard
```

**Implementation**: Create version dropdown in custom theme that links to version directories.

**Effort**: 4 hours

---

## Phase 4: Advanced Features (Ongoing)

**Priority**: Best-in-class documentation experience

### Task 4.1: Video Tutorial Placeholders

**Create `docs/src/tutorials/videos.md`**:
```markdown
# Video Tutorials

> Video tutorials are coming soon! Subscribe to our YouTube channel for updates.

## Planned Videos
1. Getting Started with AstraWeave (10 min)
2. Building Your First AI Companion (15 min)
3. Understanding the ECS Architecture (20 min)
4. Creating Adaptive Boss Battles (25 min)
5. Optimization and Profiling (15 min)
```

**Effort**: 1 hour (placeholder), 2 weeks (actual videos)

---

### Task 4.2: WebAssembly Demo Gallery

**Create interactive demos hosted at `/demos/`**:
- Fluids simulation
- Physics playground
- AI behavior visualization
- Rendering showcase

**Effort**: 1-2 weeks

---

### Task 4.3: Internationalization Framework (i18n)

**Future consideration** for Japanese, Chinese, etc. translations.

mdBook has experimental i18n support. Plan directory structure:
```
docs/src/
├── en/          # English (primary)
├── ja/          # Japanese
└── zh/          # Chinese
```

**Effort**: Ongoing community effort

---

## Implementation Timeline

### Week 1: Critical Fixes
| Day | Task | Owner | Status |
|-----|------|-------|--------|
| 1 | Task 0.1: Unify deployment actions | DevOps | |
| 1 | Task 0.2: Add .nojekyll | DevOps | |
| 1 | Task 0.3: Fix broken links | Docs | |
| 2 | Task 0.4: Validate deployment | DevOps | |
| 2 | Task 1.1: Install mdBook plugins | DevOps | |
| 3 | Task 1.2: Create 404 page | Docs | |
| 3 | Task 1.3: API docs landing page | Docs | |
| 4 | Task 1.4: Copy-to-clipboard | Frontend | |
| 5 | Task 1.5: Link benchmark dashboard | Docs | |

### Week 2-3: Content Expansion
| Task | Owner | Status |
|------|-------|--------|
| Task 2.1: Core systems pages | Core Team | |
| Task 2.2: Development guides | Contributors | |
| Task 2.3: Getting started stubs | Docs | |
| Task 2.4: Glossary | Docs | |

### Week 4: Polish
| Task | Owner | Status |
|------|-------|--------|
| Task 3.1: SEO optimization | Marketing | |
| Task 3.2: Sitemap | DevOps | |
| Task 3.3: Analytics | DevOps | |
| Task 3.4: Open Graph tags | Marketing | |
| Task 3.5: Link checking CI | DevOps | |

### Month 2+: Advanced Features
| Task | Owner | Status |
|------|-------|--------|
| Task 3.6: Version selector | DevOps | |
| Task 4.1: Video tutorials | Community | |
| Task 4.2: WASM demos | Web Dev | |

---

## Success Metrics

### Quantitative Goals
| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| **Broken Links** | 5+ | 0 | Week 1 |
| **Stub Pages** | ~29 | <5 | Week 3 |
| **Content Completeness** | 40% | 85% | Month 2 |
| **Core Web Vitals** | Unknown | Pass | Week 2 |
| **Search Result Ranking** | N/A | Top 3 for "AI native game engine" | Month 3 |

### Qualitative Goals
- Professional appearance comparable to Bevy, Tokio documentation
- Zero deployment conflicts
- Complete user journeys for all 4 audiences
- Clear, navigable structure

---

## Comparison to Industry Standards

| Feature | Bevy | Tokio | AstraWeave (Now) | AstraWeave (Target) |
|---------|------|-------|------------------|---------------------|
| mdBook plugins | 4+ | 3+ | **0** | 4+ |
| Custom 404 | Yes | Yes | **No** | Yes |
| Version selector | Yes | Yes | **No** | Yes |
| Search quality | Advanced | Advanced | **Basic** | Advanced |
| Code copy button | Yes | Yes | **No** | Yes |
| Analytics | Yes | Yes | **No** | Yes |
| Sitemap | Yes | Yes | **No** | Yes |
| Deployment stability | Stable | Stable | **CONFLICT** | Stable |

**Current Grade**: D+ (Functional but fragile)  
**After Phase 0-1**: B+ (Production-ready)  
**After Phase 2-3**: A (Best-in-class)

---

## Appendix: File Changes Summary

### Files to Create
- `docs/src/404.md`
- `docs/src/veilweaver-placeholder.md`
- `docs/src/reference/glossary.md`
- `docs/theme/clipboard.js`
- `docs/theme/clipboard.css`
- `docs/src/robots.txt`

### Files to Modify
- `.github/workflows/docs.yml`
- `.github/workflows/benchmark-dashboard.yml`
- `docs/book.toml`
- `docs/src/SUMMARY.md`
- `docs/src/api/index.md`
- All `docs/src/core-systems/*.md` files
- All `docs/src/dev/*.md` files
- `docs/src/getting-started/first-companion.md`
- `docs/src/getting-started/requirements.md`

### Files to Delete/Archive
- Remove broken Veilweaver links from SUMMARY.md (lines 55-58)
- Remove broken roadmap link from SUMMARY.md (line 85)

---

## Conclusion

This plan transforms AstraWeave's GitHub Pages from a basic mdBook deployment with critical infrastructure conflicts into a **production-grade documentation site** comparable to the best Rust ecosystem projects.

**Key Takeaways**:
1. **Fix deployment conflict IMMEDIATELY** (15 min) - Prevents content destruction
2. **Install mdBook plugins** (1 hour) - Enables diagrams, callouts, link checking
3. **Fill content gaps** (2-3 weeks) - Critical for user experience
4. **Add production features** (ongoing) - SEO, analytics, versioning

**Next Step**: Begin with Phase 0, Task 0.1 - Unify deployment actions.

---

*Document generated by Multi-Agent Analysis (Explorer, Auditor, Maintainer, ToT-Reasoner)*  
*AstraWeave AI-Native Game Engine*
