# AstraWeave ECS Refactor Plan: Archetype Integration

**Objective:** Fully integrate the archetype-based storage model into the `astraweave-ecs` crate to create a performant, robust, and ergonomic ECS foundation.

**Current State:** The `World` struct uses an inefficient, component-centric `HashMap` storage, while a more advanced but unused `ArchetypeStorage` implementation exists in `archetype.rs`.

**Proposed Plan:**

1.  **Integrate `ArchetypeStorage` into `World`:**
    *   **Action:** Replace the `comps` and `next` fields in `World` with an `ArchetypeStorage` instance and a `next_entity_id` counter.
    *   **File:** `astraweave-ecs/src/lib.rs`

2.  **Refactor `World` Methods:**
    *   **Action:** Rewrite `spawn`, `insert`, `get`, `get_mut`, `remove`, and other methods to use the `ArchetypeStorage`.
    *   **Details:**
        *   `insert`: This will be the most complex change. It must handle moving an entity between archetypes when its component signature changes.
        *   `get`/`get_mut`: These will now delegate to `ArchetypeStorage`, which will find the entity's archetype and then retrieve the component from the appropriate column.
    *   **File:** `astraweave-ecs/src/lib.rs`

3.  **Update `SystemParam` Queries:**
    *   **Action:** Refactor `Query`, `QueryMut`, `QueryTuple`, and `QueryTupleMut` to iterate over archetypes instead of the old component maps.
    *   **Details:** This will eliminate the need for `unsafe` blocks and raw pointers in the query structs, leading to safer code. Iteration will be significantly faster as it will scan contiguous memory blocks within archetypes.
    *   **File:** `astraweave-ecs/src/system_param.rs`

4.  **Expand Query Capabilities:**
    *   **Action:** With the archetype model in place, add support for more advanced query filters.
    *   **Features to Add:**
        *   `With<T>`: Filter for entities that have a component `T` without querying its data.
        *   `Without<T>`: Filter for entities that do not have component `T`.
        *   `Added<T>`: Filter for entities where component `T` was just added.
        *   `Changed<T>`: Filter for entities where component `T` has changed.
    *   **Files:** `astraweave-ecs/src/system_param.rs`, `astraweave-ecs/src/archetype.rs`

5.  **Add Comprehensive Tests:**
    *   **Action:** Write new unit and integration tests to validate all aspects of the new archetype-based implementation.
    *   **Focus Areas:**
        *   Correct entity movement between archetypes.
        *   Performance of queries.
        *   Correctness of all new query filters.
        *   Determinism of iteration.
    *   **File:** `astraweave-ecs/src/tests.rs` (or a new test file).

This refactor is a critical step towards fulfilling the "production-ready" promise of the engine's core.
