# AstraWeave ECS: Comprehensive Redesign Plan

**Date**: October 13, 2025  
**Goal**: Make AstraWeave ECS competitive with state-of-the-art game engine ECS systems (Bevy 0.15, Unity DOTS, Flecs 4.0)  
**Current Performance**: 4.52ms @ 221 FPS (1,000 entities)  
**Target Performance**: 2.70ms @ 370 FPS (Week 8 baseline), scalable to 10,000+ entities  

---

## 📊 Current Architecture Analysis

### Strengths ✅
1. **Archetype-based storage** - Cache-friendly iteration (like Bevy/Flecs)
2. **Deterministic execution** - Fixed schedules, ordered iteration (essential for AI)
3. **Event system** - Reactive behaviors for AI perception
4. **Type-safe queries** - Compile-time component access

### Critical Weaknesses ❌

#### 1. **Component Storage: Box<dyn Any> is 5-10× slower**
```rust
// Current (SLOW):
pub components: HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>,
//                                    ^^^^^^^^ Heap allocation per component!
//                                             Pointer indirection on every access
//                                             Cache misses galore

// Bevy (FAST):
pub components: HashMap<TypeId, Column>,
// where Column = BlobVec (type-erased contiguous memory)
// Single allocation, cache-friendly, SIMD-able
```

**Impact**:
- `world.get_mut::<Position>(entity)` = **O(log n) + heap deref + downcast**
- Bevy: O(1) array access (archetype index → component slice)
- **5-10× performance penalty** for every component access

#### 2. **Entity Lookup: BTreeMap is O(log n)**
```rust
// Current (SLOW):
pub entities: BTreeMap<Entity, usize>, // Entity → row index
// Every query iteration: O(log n) lookup per entity
// entities_vec(): allocates Vec every time!

// Bevy (FAST):
pub entities: Vec<Entity>,             // Direct indexing
pub entity_table: Table<Entity, usize> // O(1) reverse lookup
```

**Impact**:
- Query iteration: **1,000 entities × O(log n) = 10,000 ops**
- Bevy: 1,000 entities × O(1) = 1,000 ops
- **10× performance penalty** for queries

#### 3. **Query2Mut: Multiple World Derefs**
```rust
// Current (SLOW - Action 32 discovered):
let world_ref1 = unsafe { &mut *self.world }; // Entity lookup
let world_ref2 = unsafe { &mut *self.world }; // Mutable component
let world_ref3 = unsafe { &*self.world };     // Immutable component
// 3× pointer derefs = 600-800ns overhead per entity!

// Bevy (FAST):
fn system(mut positions: Query<&mut Position>, velocities: Query<&Velocity>) {
    // Compile-time borrow splitting via SystemParam
    // Zero runtime overhead!
}
```

**Impact**:
- Query2Mut: **+70% overhead** (1,700µs vs 1,000µs)
- Bevy: **0% overhead** (compile-time borrow checking)

#### 4. **Archetype Moves: Full Component Copy**
```rust
// Current (SLOW):
fn move_entity_to_new_archetype() {
    // 1. Remove all components from old archetype (O(C) copies)
    let components = old_archetype.remove_entity_components(entity);
    // 2. Move to new archetype (O(C) copies)
    new_archetype.add_entity(entity, components);
    // Total: 2 × O(C) copies + 2 × HashMap lookups
}

// Bevy (FAST):
// Uses "edge" graph between archetypes
// Precomputes add/remove transitions
// Direct memcpy between columns (O(1) swap_remove)
```

**Impact**:
- Adding/removing components: **2-5ms for 1,000 entities**
- Bevy: **0.1-0.5ms** (10× faster)

#### 5. **No Parallel Execution**
```rust
// Current (SEQUENTIAL):
pub fn run(&self, world: &mut World) {
    for stage in &self.stages {
        for system in &stage.systems {
            (system)(world); // Sequential execution!
        }
    }
}

// Bevy (PARALLEL):
// Analyzes system dependencies at compile time
// Runs independent systems in parallel via Rayon
// 2-4× speedup on multi-core CPUs
```

#### 6. **No Change Detection**
- No way to detect "has component X changed since last frame?"
- Forces full iteration every frame
- Bevy/DOTS: `Changed<T>`, `Added<T>`, `Removed<T>` filters

---

## 🎯 Design Goals

### Performance Targets
1. **Query Iteration**: <100ns per entity (currently ~1,000ns)
2. **Component Access**: <10ns (currently ~100ns with Box<dyn Any>)
3. **Archetype Moves**: <500ns per entity (currently ~2,000ns)
4. **Parallel Systems**: 2-4× speedup on 4+ cores
5. **Scalability**: 10,000 entities @ 60 FPS, 100,000+ supported

### Stability Requirements
1. **Deterministic Execution**: Maintained for AI/networking
2. **Memory Safety**: No UB, validated unsafe blocks
3. **API Stability**: Minimize breaking changes
4. **Backward Compatibility**: Provide migration path

### Feature Parity with SOTA
| Feature | Bevy 0.15 | Unity DOTS | Flecs 4.0 | AstraWeave Current | AstraWeave Target |
|---------|-----------|------------|-----------|-------------------|-------------------|
| Archetype Storage | ✅ | ✅ | ✅ | ✅ | ✅ |
| Contiguous Components | ✅ | ✅ | ✅ | ❌ | ✅ |
| O(1) Entity Lookup | ✅ | ✅ | ✅ | ❌ | ✅ |
| Zero-Cost Queries | ✅ | ✅ | ⚠️ | ❌ | ✅ |
| Parallel Systems | ✅ | ✅ | ✅ | ❌ | ✅ |
| Change Detection | ✅ | ✅ | ✅ | ❌ | ✅ |
| Archetype Edges | ✅ | ✅ | ❌ | ❌ | ✅ |
| SystemParam DSL | ✅ | ❌ | ⚠️ | ❌ | ✅ |
| Deterministic | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ |

---

## 🏗️ Redesign Architecture

### Phase 1: Storage Layer Rewrite (High Priority)

#### 1.1: BlobVec - Type-Erased Contiguous Storage
```rust
/// Contiguous, type-erased storage (like Bevy's BlobVec)
pub struct BlobVec {
    data: NonNull<u8>,      // Raw memory
    capacity: usize,
    len: usize,
    item_layout: Layout,    // Component size/align
    drop: unsafe fn(*mut u8), // Drop function for cleanup
}

impl BlobVec {
    /// Push component without knowing its type at compile time
    pub unsafe fn push<T: Component>(&mut self, value: T) {
        // Reserve space, write bytes, increment len
        // Single allocation, no Box indirection!
    }
    
    /// Get component slice (SIMD-friendly!)
    pub unsafe fn get_slice<T: Component>(&self) -> &[T] {
        std::slice::from_raw_parts(
            self.data.as_ptr() as *const T,
            self.len
        )
    }
    
    /// Get mutable component slice
    pub unsafe fn get_slice_mut<T: Component>(&mut self) -> &mut [T] {
        std::slice::from_raw_parts_mut(
            self.data.as_ptr() as *mut T,
            self.len
        )
    }
}
```

**Benefits**:
- **10× faster** component access (no Box, no downcast)
- **SIMD-friendly**: Contiguous memory enables auto-vectorization
- **Cache-friendly**: All components of same type packed together
- **Memory-efficient**: Single allocation vs N allocations

#### 1.2: Table - Sparse Set for O(1) Lookups
```rust
/// Sparse set for O(1) entity → component index
pub struct SparseSet<T> {
    sparse: Vec<Option<usize>>, // Entity ID → dense index
    dense: Vec<T>,              // Packed data
}

impl<T> SparseSet<T> {
    pub fn insert(&mut self, entity: Entity, value: T) {
        let id = entity.id() as usize;
        if id >= self.sparse.len() {
            self.sparse.resize(id + 1, None);
        }
        let dense_idx = self.dense.len();
        self.dense.push(value);
        self.sparse[id] = Some(dense_idx);
    }
    
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let id = entity.id() as usize;
        let dense_idx = *self.sparse.get(id)??;
        self.dense.get(dense_idx)
    }
}
```

**Benefits**:
- **O(1) entity lookup** (vs O(log n) BTreeMap)
- **Packed storage**: Dense array for cache-friendly iteration
- **Proven pattern**: Used by EnTT, Flecs, Bevy

#### 1.3: Archetype Redesign
```rust
pub struct Archetype {
    pub id: ArchetypeId,
    pub signature: ArchetypeSignature,
    
    // NEW: Contiguous storage
    entities: Vec<Entity>,           // Packed entity list
    entity_index: SparseSet<usize>,  // Entity → row (O(1))
    
    // NEW: BlobVec per component type
    components: HashMap<TypeId, BlobVec>,
    
    // NEW: Archetype graph edges (Bevy-style)
    edges: HashMap<TypeId, ArchetypeEdge>,
}

pub struct ArchetypeEdge {
    add: ArchetypeId,    // Target when adding component
    remove: ArchetypeId, // Target when removing component
}
```

**Benefits**:
- **100× faster** entity lookup (O(1) vs O(log n))
- **10× faster** component iteration (contiguous vs Box)
- **Precomputed edges**: Archetype moves become O(1) lookup

### Phase 2: Query System Rewrite (High Priority)

#### 2.1: SystemParam Trait - Compile-Time Borrow Splitting
```rust
/// Trait for extracting system parameters from World
pub trait SystemParam: Sized {
    type State: Send + Sync + 'static;
    type Item<'world, 'state>: SystemParam;
    
    fn init_state(world: &mut World) -> Self::State;
    fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        world: &'world World,
    ) -> Self::Item<'world, 'state>;
}

// Implementation for Query<T>
impl<T: Component> SystemParam for Query<'_, T> {
    type State = QueryState<T>;
    // ... compile-time borrow checking!
}
```

#### 2.2: Zero-Cost Query Iteration
```rust
pub struct Query<'world, T: Component> {
    archetypes: &'world [&'world Archetype],
    marker: PhantomData<T>,
}

impl<'world, T: Component> Query<'world, T> {
    pub fn iter(&self) -> QueryIter<'world, T> {
        QueryIter {
            archetypes: self.archetypes,
            arch_idx: 0,
            component_slice: &[],  // Direct slice access!
            slice_idx: 0,
        }
    }
}

pub struct QueryIter<'world, T: Component> {
    archetypes: &'world [&'world Archetype],
    arch_idx: usize,
    component_slice: &'world [T],  // Direct memory access!
    slice_idx: usize,
}

impl<'world, T: Component> Iterator for QueryIter<'world, T> {
    type Item = (Entity, &'world T);
    
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.slice_idx < self.component_slice.len() {
                let entity = self.archetypes[self.arch_idx].entities[self.slice_idx];
                let component = &self.component_slice[self.slice_idx];
                self.slice_idx += 1;
                return Some((entity, component));
            }
            
            // Move to next archetype
            self.arch_idx += 1;
            if self.arch_idx >= self.archetypes.len() {
                return None;
            }
            
            // Get component slice from archetype
            let arch = self.archetypes[self.arch_idx];
            self.component_slice = unsafe {
                arch.get_component_slice::<T>()
            };
            self.slice_idx = 0;
        }
    }
}
```

**Benefits**:
- **Zero runtime overhead**: Direct slice iteration
- **SIMD-friendly**: Compiler can auto-vectorize
- **No entity lookups**: Entities and components iterated together
- **100× faster** than current implementation

#### 2.3: Mutable Queries without Overhead
```rust
pub struct QueryMut<'world, T: Component> {
    archetypes: &'world [&'world Archetype],
    marker: PhantomData<T>,
}

impl<'world, T: Component> QueryMut<'world, T> {
    pub fn iter_mut(&mut self) -> QueryIterMut<'world, T> {
        QueryIterMut {
            archetypes: self.archetypes,
            arch_idx: 0,
            component_slice: &mut [],  // Mutable slice!
            slice_idx: 0,
        }
    }
}

// Implementation similar to QueryIter but with &mut [T]
```

**Benefits**:
- **No world pointer derefs** (compile-time borrow)
- **Zero overhead** mutation
- **70% faster** than Query2Mut (eliminates overhead from Action 32)

### Phase 3: Parallel System Execution (Medium Priority)

#### 3.1: System Dependency Analysis
```rust
pub struct SystemMeta {
    pub reads: Vec<TypeId>,   // Components read
    pub writes: Vec<TypeId>,  // Components written
    pub resources: Vec<TypeId>, // Resources accessed
}

pub struct SystemGraph {
    systems: Vec<Box<dyn System>>,
    dependencies: Vec<Vec<usize>>, // DAG
}

impl SystemGraph {
    pub fn analyze_dependencies(&mut self) {
        // Compute which systems can run in parallel
        // Two systems are independent if:
        // 1. No overlapping writes
        // 2. Reads don't conflict with writes
    }
    
    pub fn run_parallel(&mut self, world: &mut World) {
        // Execute systems in parallel using Rayon
        // Respect dependencies computed above
    }
}
```

#### 3.2: Rayon Integration
```rust
impl Schedule {
    pub fn run_parallel(&self, world: &mut World) {
        for stage in &self.stages {
            // Partition systems into independent batches
            let batches = self.compute_batches(&stage.systems);
            
            for batch in batches {
                // Run batch in parallel
                batch.par_iter().for_each(|system| {
                    system.run(world);
                });
            }
        }
    }
}
```

**Benefits**:
- **2-4× speedup** on multi-core CPUs (4+ cores)
- **Maintains determinism**: Execute batches in order
- **Automatic parallelization**: No user code changes

### Phase 4: Change Detection (Medium Priority)

#### 4.1: Component Ticks
```rust
pub struct ComponentTicks {
    added: Tick,     // When component was added
    changed: Tick,   // Last mutation
}

pub struct Tick(u32);

impl World {
    pub fn increment_tick(&mut self) {
        self.current_tick.0 += 1;
    }
}

pub struct Changed<T>(PhantomData<T>);

impl<T: Component> SystemParam for Query<'_, Changed<T>> {
    // Only iterate entities where T.changed > last_system_tick
}
```

**Benefits**:
- **10-100× faster** for sparse updates (skip unchanged entities)
- **Event-driven systems**: React only to changes
- **AI optimization**: Update perception only when world changes

### Phase 5: Archetype Graph Edges (Low Priority)

```rust
impl Archetype {
    pub fn get_add_edge(&self, component: TypeId) -> Option<ArchetypeId> {
        self.edges.get(&component).map(|e| e.add)
    }
    
    pub fn get_remove_edge(&self, component: TypeId) -> Option<ArchetypeId> {
        self.edges.get(&component).map(|e| e.remove)
    }
}

impl World {
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        let old_arch = self.entity_archetype(entity);
        
        // Fast path: Use precomputed edge
        if let Some(new_arch) = old_arch.get_add_edge(TypeId::of::<T>()) {
            self.move_entity_fast(entity, new_arch, component);
            return;
        }
        
        // Slow path: Compute new archetype, cache edge
        let new_arch = self.compute_archetype_after_add::<T>(old_arch);
        old_arch.add_edge(TypeId::of::<T>(), new_arch);
        self.move_entity_fast(entity, new_arch, component);
    }
}
```

**Benefits**:
- **O(1) archetype moves** (vs O(log n) HashMap lookup)
- **Amortized cost**: First move computes edge, subsequent moves are free
- **5-10× faster** component add/remove

---

## 📈 Expected Performance Improvements

### Component Access
| Operation | Current | Redesigned | Speedup |
|-----------|---------|------------|---------|
| `world.get::<T>(entity)` | 100ns | 10ns | **10×** |
| `world.get_mut::<T>(entity)` | 120ns | 15ns | **8×** |
| Query iteration (1k entities) | 1,000µs | 100µs | **10×** |
| Query2 iteration | 1,270µs | 120µs | **10.6×** |

### Movement System Performance
| Implementation | Time | FPS | vs Current |
|----------------|------|-----|------------|
| Current (Action 32) | 1,000µs | 221 FPS | Baseline |
| After BlobVec | 300µs | 500+ FPS | **3.3×** |
| After Zero-Cost Query | 100µs | 1000+ FPS | **10×** |

### Overall Frame Time
| Scenario | Current | Redesigned | Target |
|----------|---------|------------|--------|
| 1k entities | 4.52ms | 1.5ms | 2.70ms ✅ |
| 5k entities | ~20ms | ~6ms | <16.67ms ✅ |
| 10k entities | ~40ms | ~10ms | <16.67ms ✅ |

---

## 🚀 Implementation Roadmap

### Week 10 (October 13-18, 2025) - Foundation
**Priority**: Critical path items

1. **BlobVec Implementation** (Day 1-2, 8h)
   - Core BlobVec with push/get_slice
   - Safety tests (miri validation)
   - Benchmark vs Box<dyn Any>

2. **SparseSet Implementation** (Day 2, 4h)
   - Generic SparseSet<T>
   - O(1) insert/get/remove
   - Benchmark vs BTreeMap

3. **Archetype Storage Migration** (Day 3-4, 12h)
   - Replace HashMap<TypeId, Vec<Box>> with HashMap<TypeId, BlobVec>
   - Replace BTreeMap<Entity, usize> with SparseSet
   - Update all World methods (get, get_mut, insert, remove)
   - **Critical**: Maintain API compatibility

4. **Query Rewrite - Phase 1** (Day 4-5, 8h)
   - Implement QueryIter with slice-based iteration
   - Zero-cost Query<&T> and Query<&mut T>
   - Benchmark vs current Query/Query2

**Deliverable**: 3-5× performance improvement on movement system

### Week 11 (October 19-25, 2025) - SystemParam DSL
**Priority**: High (enables ergonomic systems)

5. **SystemParam Trait** (Day 1-2, 10h)
   - Core trait definition
   - QueryState implementation
   - ResMut/Res implementations

6. **System Signature Rewrite** (Day 2-3, 8h)
   - Convert `fn(world: &mut World)` → `fn(query: Query<&Position>)`
   - Compile-time borrow checking
   - Update profiling_demo

7. **Multi-Component Queries** (Day 3-4, 8h)
   - Query<(&Position, &Velocity)>
   - Query<(&mut Position, &Velocity)>
   - With/Without filters

**Deliverable**: Ergonomic system API with zero overhead

### Week 12 (October 26 - November 1, 2025) - Parallel Execution
**Priority**: Medium (2-4× speedup on multi-core)

8. **System Metadata** (Day 1, 4h)
   - Extract reads/writes from SystemParam
   - Build dependency graph

9. **Parallel Executor** (Day 2-3, 10h)
   - Rayon integration
   - Batch scheduling
   - Deterministic execution order

10. **Stress Testing** (Day 4-5, 8h)
    - 10k, 50k, 100k entity benchmarks
    - Multi-threaded validation
    - Determinism tests

**Deliverable**: 2-4× frame time reduction on 4+ cores

### Week 13-14 (November 2-15, 2025) - Change Detection
**Priority**: Low (optimization for sparse updates)

11. **Component Ticks** (Day 1-2, 8h)
    - Add Tick to BlobVec
    - Track added/changed/removed

12. **Filter Queries** (Day 2-3, 8h)
    - Changed<T>, Added<T>, Removed<T>
    - Without<T>, With<T>

13. **Event-Driven AI** (Day 4-5, 8h)
    - Perception system uses Changed<Position>
    - AI planning triggers on Changed<AIAgent>

**Deliverable**: 10-100× speedup for sparse update patterns

---

## ✅ Validation Strategy

### Performance Benchmarks
1. **Micro-benchmarks**:
   - Component access (get/get_mut)
   - Query iteration (1k, 10k, 100k entities)
   - Archetype moves
   - System scheduling

2. **Integration benchmarks**:
   - profiling_demo (current: 4.52ms → target: 1.5ms)
   - hello_companion
   - Veilweaver demo

3. **Scaling tests**:
   - 1k, 5k, 10k, 50k, 100k entities
   - Linear scaling validation
   - Memory usage profiling

### Safety Validation
1. **Miri**: Run all tests under Miri (detects UB)
2. **ASAN**: Address sanitizer for memory leaks
3. **TSAN**: Thread sanitizer for data races
4. **Fuzzing**: Property-based testing (proptest)

### Determinism Validation
1. **Replay tests**: Same inputs → same outputs
2. **Checksum validation**: Hash world state every frame
3. **Multi-platform**: Windows/Linux/macOS consistency

---

## 🎓 Key Architectural Decisions

### 1. BlobVec over Box<dyn Any>
**Rationale**: Bevy, DOTS, EnTT all use type-erased contiguous storage. Proven 10× faster.

**Tradeoffs**:
- ✅ 10× faster iteration, SIMD-friendly
- ❌ More unsafe code (validated by Miri)
- ❌ Harder to debug (raw pointers)

**Decision**: Worth the tradeoff - performance is critical.

### 2. SparseSet over BTreeMap
**Rationale**: O(1) vs O(log n) for entity lookups. Flecs and EnTT use sparse sets.

**Tradeoffs**:
- ✅ 100× faster lookups
- ❌ Higher memory usage (sparse array)
- ❌ Fragmentation for high entity IDs

**Decision**: Use sparse set with compaction strategy.

### 3. SystemParam DSL over Raw World Access
**Rationale**: Bevy's API is ergonomic and enables compile-time borrow checking.

**Tradeoffs**:
- ✅ Zero runtime overhead
- ✅ Ergonomic API
- ❌ More complex implementation
- ❌ Learning curve for users

**Decision**: Essential for competitive ECS - implement in Week 11.

### 4. Deterministic Parallel Execution
**Rationale**: AI/networking requires determinism, but parallelism gives 2-4× speedup.

**Tradeoffs**:
- ✅ 2-4× speedup maintained
- ✅ Determinism preserved (execute batches in order)
- ❌ Less parallelism than Bevy (more conservative)

**Decision**: Determinism is non-negotiable for AstraWeave's AI-first focus.

---

## 📚 References

### Bevy ECS (Primary Reference)
- **Architecture**: https://bevyengine.org/learn/book/getting-started/ecs/
- **Source**: https://github.com/bevyengine/bevy/tree/main/crates/bevy_ecs
- **Key Files**:
  - `blob_vec.rs`: Type-erased storage
  - `system_param.rs`: Compile-time borrow checking
  - `archetype.rs`: Archetype graph with edges

### Unity DOTS
- **Documentation**: https://docs.unity3d.com/Packages/com.unity.entities@1.0/
- **Key Concepts**: Chunk iteration, structural changes, Burst compiler

### Flecs 4.0
- **Source**: https://github.com/SanderMertens/flecs
- **Key Concepts**: Sparse sets, relationship queries, prefabs

### EnTT
- **Source**: https://github.com/skypjack/entt
- **Key Concepts**: Sparse sets, component pools, observers

---

## 🎯 Success Criteria

### Performance (Quantitative)
- ✅ Component access: <20ns (currently ~100ns)
- ✅ Query iteration: <150ns/entity (currently ~1,000ns)
- ✅ Frame time @ 1k entities: <2.0ms (currently 4.52ms)
- ✅ Scalability: 10k entities @ 60 FPS
- ✅ Parallel speedup: 2-3× on 4 cores

### Stability (Qualitative)
- ✅ Zero UB (Miri validation)
- ✅ Zero data races (TSAN validation)
- ✅ Deterministic execution (replay tests)
- ✅ Memory safe (no leaks, ASAN validation)

### Developer Experience
- ✅ Ergonomic API (SystemParam DSL)
- ✅ Compile-time errors (not runtime panics)
- ✅ Migration guide (v0.1 → v0.2)
- ✅ Comprehensive docs

---

**Next Steps**: Proceed with Week 10 implementation (BlobVec + SparseSet + Archetype migration)

---

*This document was generated entirely by AI (GitHub Copilot) as part of the AstraWeave AI-native game engine experiment.*
