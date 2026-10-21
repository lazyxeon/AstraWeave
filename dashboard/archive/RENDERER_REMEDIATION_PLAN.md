# AstraWeave Renderer Remediation Plan

**Status**: Phase 1 Complete | **Date**: November 2025
**Objective**: Unify the disconnected rendering architectures in `astraweave-render` to achieve world-class visual quality and performance.

## Executive Summary

The `astraweave-render` crate currently contains two disconnected rendering paths:
1.  **Legacy Forward Renderer** (`renderer.rs`): Currently active. Uses hardcoded shaders, basic forward rendering, and lacks advanced features.
2.  **Modern Clustered Renderer** (`clustered_forward.rs`): Currently inactive. Contains advanced lighting, compute shader culling, and modern architecture.

This plan outlines the steps to merge these paths, activating the modern features while maintaining stability.

## Phase 1: Shader Infrastructure (✅ COMPLETE)

**Goal**: Decouple shader code from Rust source to enable modularity and easier integration of complex lighting logic.

-   [x] **Audit**: Identify all hardcoded shader strings in `renderer.rs`.
-   [x] **Externalize**: Move PBR shader to `shaders/pbr.wgsl`.
-   [x] **Externalize**: Move Basic Post shader to `shaders/post_basic.wgsl`.
-   [x] **Externalize**: Move FX Post shader to `shaders/post_fx.wgsl`.
-   [x] **Verify**: Ensure `astraweave-render` compiles with externalized shaders.

**Outcome**: `renderer.rs` now uses `include_str!` for shaders, making it possible to inject clustered lighting logic without recompiling Rust code or managing massive string constants.

## Phase 2: Clustered Lighting Integration (🚧 NEXT)

**Goal**: Replace the basic point light loop in `pbr.wgsl` with the clustered lighting logic from `clustered_forward.rs`.

-   [ ] **Analyze `clustered_forward.rs`**: Understand the bind group layout and compute shader dispatch for light culling.
-   [ ] **Port Shader Logic**: Copy the light cluster lookup logic from the clustered renderer's shaders (if available) or write new WGSL to access the light grid.
-   [ ] **Update `renderer.rs`**:
    -   Add bind groups for the Light Grid (clusters) and Global Light Index list.
    -   Dispatch the light culling compute shader before the main render pass.
-   [ ] **Update `pbr.wgsl`**:
    -   Replace the `for (var i=0u; i<num_lights; i++)` loop with a cluster-aware loop.
    -   `let cluster_index = get_cluster_index(in.frag_pos);`
    -   `loop_lights_in_cluster(cluster_index, ...)`

## Phase 3: Render Graph Activation

**Goal**: Move from a hardcoded pass sequence to a data-driven Render Graph.

-   [ ] **Review `render_graph.rs`**: Assess the maturity of the existing implementation.
-   [ ] **Define Graph**: Create a graph that replicates the current `Pre-Pass -> Main -> Post` flow.
-   [ ] **Switch Execution**: Replace the manual command encoder calls in `renderer.rs` with `graph.execute()`.

## Phase 4: Nanite & Advanced Features

**Goal**: Activate the dormant `nanite` and `gi` modules.

-   [ ] **Nanite**: Connect the `virtual_geometry` module to the main render loop.
-   [ ] **GI**: Enable the Global Illumination pass in the Render Graph.

## Immediate Next Steps

1.  Analyze `clustered_forward.rs` to understand the data structures required for the GPU (Light clusters, indices).
2.  Modify `renderer.rs` to create and bind these buffers.
