# AstraWeave Renderer Audit Report
**Date:** November 18, 2025
**Auditor:** GitHub Copilot (World-Class Graphics Engineer Persona)
**Scope:** `astraweave-render` crate

## Executive Summary
The `astraweave-render` crate contains the building blocks of a high-end "World-Class" engine (Clustered Forward Rendering, Nanite-style geometry, Global Illumination, Cascaded Shadows), but these components are currently **fragmented and largely dormant**. The primary `Renderer` implementation (`renderer.rs`) uses a legacy, hardcoded forward rendering path that ignores the advanced modules available in the codebase.

To achieve "World-Class" status, the engine does not need *new* features as much as it needs **integration** of existing ones.

## Critical Weaknesses

### 1. Disconnected Rendering Architectures
- **Observation**: The codebase contains a sophisticated `ClusteredForwardRenderer` (`clustered_forward.rs`) with GPU compute culling (`megalights`), but the main `Renderer` (`renderer.rs`) uses a basic forward pass with a single directional light.
- **Impact**: The `unified_showcase` looks "basic" because it's running on the legacy path. Point lights and advanced lighting are effectively disabled (commented out in shaders).
- **Severity**: **Critical** (Blocks visual fidelity).

### 2. Primitive Shader Management
- **Observation**: Core shaders (PBR, Post-processing) are defined as hardcoded Rust string constants (`const SHADER_SRC: &str = ...`) inside `renderer.rs`.
- **Impact**: 
    - Impossible to implement shader hot-reloading.
    - Difficult to maintain or extend (no syntax highlighting, no modularity).
    - Prevents code sharing between the Clustered Lighting module and the Main Renderer.
- **Severity**: **High** (Blocks iteration speed and modularity).

### 3. Underutilized Render Graph
- **Observation**: A `graph.rs` module exists, implying a Render Graph architecture, but `renderer.rs` manually orchestrates passes (Shadows -> Main -> Post).
- **Impact**: Adding new passes (e.g., SSAO, Bloom, Transparency) increases complexity exponentially rather than linearly.
- **Severity**: **Medium** (Technical debt).

### 4. Manual Batching
- **Observation**: The renderer relies on `instances: Vec<Instance>`, suggesting manual instancing. There is no evidence of automatic batching of distinct meshes with the same material.
- **Impact**: CPU draw call overhead will bottleneck performance in complex scenes.
- **Severity**: **Medium** (Performance bottleneck).

## Remediation Plan

### Phase 1: Foundation & Shader Infrastructure (Immediate)
- **Goal**: Externalize all hardcoded shaders into `.wgsl` files.
- **Benefit**: Enables modularity, code sharing, and prepares for Clustered Lighting integration.
- **Action**: Move `SHADER_SRC`, `POST_SHADER`, etc., to `shaders/pbr.wgsl`, `shaders/post.wgsl`.

### Phase 2: Lighting Integration (Short Term)
- **Goal**: Wire `ClusteredForwardRenderer` into the main `Renderer`.
- **Benefit**: Enables 100+ dynamic lights and "next-gen" lighting visuals.
- **Action**: 
    - Update `pbr.wgsl` to include the clustered lighting logic.
    - Bind the cluster buffers in `Renderer`.

### Phase 3: Render Graph Adoption (Medium Term)
- **Goal**: Refactor `Renderer` to execute a `RenderGraph`.
- **Benefit**: Simplifies pipeline management and enables complex post-processing chains.

## Conclusion
The engine is a "Ferrari engine in a go-kart chassis." The advanced components are there, but the chassis (the main `Renderer`) isn't built to use them. The immediate priority is to clean up the shader infrastructure to allow these components to be connected.
