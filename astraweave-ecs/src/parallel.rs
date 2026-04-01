//! Parallel system execution for the ECS scheduler.
//!
//! This module provides a parallel-capable scheduler that runs non-conflicting
//! systems concurrently within each stage using Rayon. Systems declare their
//! data access (which resources/components they read and write) via [`SystemAccess`],
//! and the scheduler groups them to maximize parallelism while maintaining soundness.
//!
//! # Architecture
//!
//! - **Stages execute sequentially** (required for determinism)
//! - **Within a stage**, systems are grouped by access conflict:
//!   - Systems with disjoint access sets run in parallel
//!   - Systems with overlapping write access run sequentially
//! - **Exclusive systems** (declared with `AccessKind::Exclusive`) always run alone
//!
//! # Usage
//!
//! ```rust,ignore
//! use astraweave_ecs::parallel::*;
//! use astraweave_ecs::World;
//! use std::any::TypeId;
//!
//! fn physics_system(world: &mut World) { /* ... */ }
//! fn render_prep_system(world: &mut World) { /* ... */ }
//!
//! let mut schedule = ParallelSchedule::new();
//! schedule.add_stage("simulation");
//! schedule.add_system("simulation", SystemDescriptor::new(physics_system)
//!     .writes::<PhysicsState>());
//! schedule.add_system("simulation", SystemDescriptor::new(render_prep_system)
//!     .reads::<PhysicsState>());
//! // physics and render_prep can run in parallel (read + write on same type is conflict,
//! // but read + read is fine)
//! ```

use std::any::TypeId;
use std::collections::HashSet;

use crate::World;

/// Describes what data a system accesses.
///
/// Used by the parallel scheduler to determine which systems can run concurrently.
/// Systems with disjoint access sets are safe to run in parallel.
#[derive(Clone, Debug, Default)]
pub struct SystemAccess {
    /// Resource/component TypeIds this system reads (shared access)
    pub reads: HashSet<TypeId>,
    /// Resource/component TypeIds this system writes (exclusive access)
    pub writes: HashSet<TypeId>,
    /// If true, this system needs exclusive access to the entire World.
    /// It will always run alone, never in parallel with other systems.
    pub exclusive: bool,
}

impl SystemAccess {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if this system's access conflicts with another.
    /// Conflict occurs when one system writes to something the other reads or writes.
    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        if self.exclusive || other.exclusive {
            return true;
        }
        // Write-write conflict
        if !self.writes.is_disjoint(&other.writes) {
            return true;
        }
        // Write-read conflict (either direction)
        if !self.writes.is_disjoint(&other.reads) {
            return true;
        }
        if !self.reads.is_disjoint(&other.writes) {
            return true;
        }
        false
    }
}

/// A system function paired with its access metadata.
pub struct SystemDescriptor {
    pub func: fn(&mut World),
    pub access: SystemAccess,
    pub name: &'static str,
}

impl SystemDescriptor {
    /// Create a new system descriptor with no declared access (defaults to exclusive).
    ///
    /// Systems without declared access are treated as exclusive for safety —
    /// they run alone. Call `.reads::<T>()` / `.writes::<T>()` to declare access
    /// and enable parallel execution.
    pub fn new(func: fn(&mut World)) -> Self {
        Self {
            func,
            access: SystemAccess {
                exclusive: true, // Safe default: run alone until access is declared
                ..Default::default()
            },
            name: "",
        }
    }

    /// Create a system that has declared its access (not exclusive by default).
    pub fn with_access(func: fn(&mut World), access: SystemAccess) -> Self {
        Self {
            func,
            access,
            name: "",
        }
    }

    /// Set the debug name.
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// Declare that this system reads a resource/component type.
    pub fn reads<T: 'static>(mut self) -> Self {
        self.access.exclusive = false;
        self.access.reads.insert(TypeId::of::<T>());
        self
    }

    /// Declare that this system writes a resource/component type.
    pub fn writes<T: 'static>(mut self) -> Self {
        self.access.exclusive = false;
        self.access.writes.insert(TypeId::of::<T>());
        self
    }

    /// Mark this system as needing exclusive World access.
    pub fn exclusive(mut self) -> Self {
        self.access.exclusive = true;
        self
    }
}

/// A stage containing system descriptors.
pub struct ParallelStage {
    pub name: &'static str,
    pub systems: Vec<SystemDescriptor>,
}

/// A schedule that can execute systems in parallel within each stage.
///
/// Stages execute sequentially (deterministic). Within each stage, systems are
/// grouped by access conflict — non-conflicting systems run in parallel.
pub struct ParallelSchedule {
    pub stages: Vec<ParallelStage>,
}

impl Default for ParallelSchedule {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelSchedule {
    pub fn new() -> Self {
        Self { stages: vec![] }
    }

    pub fn with_stage(mut self, name: &'static str) -> Self {
        self.stages.push(ParallelStage {
            name,
            systems: vec![],
        });
        self
    }

    pub fn add_stage(&mut self, name: &'static str) {
        self.stages.push(ParallelStage {
            name,
            systems: vec![],
        });
    }

    pub fn add_system(&mut self, stage: &'static str, desc: SystemDescriptor) {
        if let Some(s) = self.stages.iter_mut().find(|s| s.name == stage) {
            s.systems.push(desc);
        }
    }

    /// Build parallel execution groups for a stage.
    ///
    /// Returns a list of groups where each group contains system indices that
    /// can safely run in parallel. Groups themselves must run sequentially.
    ///
    /// Algorithm: greedy coloring — assign each system to the first group it
    /// doesn't conflict with. O(n²) in system count per stage, which is fine
    /// since stages typically have <20 systems.
    fn build_groups(systems: &[SystemDescriptor]) -> Vec<Vec<usize>> {
        let mut groups: Vec<Vec<usize>> = vec![];

        for (i, sys) in systems.iter().enumerate() {
            let mut placed = false;
            for group in &mut groups {
                // Check if this system conflicts with any system already in the group
                let conflicts = group.iter().any(|&j| sys.access.conflicts_with(&systems[j].access));
                if !conflicts {
                    group.push(i);
                    placed = true;
                    break;
                }
            }
            if !placed {
                groups.push(vec![i]);
            }
        }

        groups
    }

    /// Run all stages sequentially. Within each stage, non-conflicting systems
    /// run in parallel (when the `parallel` feature is enabled).
    pub fn run(&self, world: &mut World) {
        #[cfg(feature = "profiling")]
        astraweave_profiling::span!("ECS::ParallelSchedule::run");

        for stage in &self.stages {
            if stage.systems.is_empty() {
                continue;
            }

            let groups = Self::build_groups(&stage.systems);

            for group in &groups {
                if group.len() == 1 {
                    // Single system — run directly, no overhead
                    (stage.systems[group[0]].func)(world);
                } else {
                    // Multiple non-conflicting systems — run in parallel
                    self.run_group_parallel(&stage.systems, group, world);
                }
            }
        }
    }

    /// Run a group of non-conflicting systems in parallel.
    ///
    /// # Safety Argument
    ///
    /// This uses `unsafe` to share `&mut World` across threads. Soundness relies on:
    /// 1. All systems in the group have been verified to have disjoint access sets
    ///    (no write-write or read-write conflicts on the same TypeId).
    /// 2. Rayon's `scope` ensures all spawned tasks complete before returning.
    /// 3. The World pointer remains valid for the entire scope duration.
    fn run_group_parallel(
        &self,
        systems: &[SystemDescriptor],
        group: &[usize],
        world: &mut World,
    ) {
        #[cfg(feature = "parallel")]
        {
            let world_ptr = world as *mut World;

            // SAFETY: All systems in this group have been verified by build_groups()
            // to have non-conflicting access sets. Each system accesses disjoint
            // resources/components, so concurrent &mut World access is sound.
            // The raw pointer is valid for the rayon::scope duration.
            rayon::scope(|s| {
                for &idx in group {
                    let func = systems[idx].func;
                    let ptr = world_ptr;
                    s.spawn(move |_| {
                        // SAFETY: See above — disjoint access guaranteed by scheduler.
                        let world_ref = unsafe { &mut *ptr };
                        func(world_ref);
                    });
                }
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            // Fallback: sequential execution when rayon is not available
            for &idx in group {
                (systems[idx].func)(world);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Marker types for access declarations
    struct PhysicsData;
    struct RenderData;
    struct AudioData;

    #[test]
    fn test_system_access_no_conflict() {
        let a = SystemAccess {
            reads: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            writes: HashSet::new(),
            exclusive: false,
        };
        let b = SystemAccess {
            reads: [TypeId::of::<RenderData>()].into_iter().collect(),
            writes: HashSet::new(),
            exclusive: false,
        };
        assert!(!a.conflicts_with(&b));
    }

    #[test]
    fn test_system_access_read_read_no_conflict() {
        let a = SystemAccess {
            reads: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            writes: HashSet::new(),
            exclusive: false,
        };
        let b = SystemAccess {
            reads: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            writes: HashSet::new(),
            exclusive: false,
        };
        assert!(!a.conflicts_with(&b), "read-read should not conflict");
    }

    #[test]
    fn test_system_access_write_write_conflict() {
        let a = SystemAccess {
            reads: HashSet::new(),
            writes: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            exclusive: false,
        };
        let b = SystemAccess {
            reads: HashSet::new(),
            writes: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            exclusive: false,
        };
        assert!(a.conflicts_with(&b), "write-write should conflict");
    }

    #[test]
    fn test_system_access_read_write_conflict() {
        let a = SystemAccess {
            reads: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            writes: HashSet::new(),
            exclusive: false,
        };
        let b = SystemAccess {
            reads: HashSet::new(),
            writes: [TypeId::of::<PhysicsData>()].into_iter().collect(),
            exclusive: false,
        };
        assert!(a.conflicts_with(&b), "read-write should conflict");
    }

    #[test]
    fn test_system_access_exclusive_always_conflicts() {
        let a = SystemAccess {
            exclusive: true,
            ..Default::default()
        };
        let b = SystemAccess::default();
        assert!(a.conflicts_with(&b), "exclusive should conflict with anything");
    }

    #[test]
    fn test_build_groups_all_independent() {
        let systems = vec![
            SystemDescriptor::new(|_| {}).reads::<PhysicsData>(),
            SystemDescriptor::new(|_| {}).reads::<RenderData>(),
            SystemDescriptor::new(|_| {}).reads::<AudioData>(),
        ];
        let groups = ParallelSchedule::build_groups(&systems);
        // All systems read different types — should be 1 group with all 3
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn test_build_groups_all_conflicting() {
        let systems = vec![
            SystemDescriptor::new(|_| {}).writes::<PhysicsData>(),
            SystemDescriptor::new(|_| {}).writes::<PhysicsData>(),
            SystemDescriptor::new(|_| {}).writes::<PhysicsData>(),
        ];
        let groups = ParallelSchedule::build_groups(&systems);
        // All write the same type — each in its own group
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_build_groups_mixed() {
        let systems = vec![
            SystemDescriptor::new(|_| {}).writes::<PhysicsData>().named("physics"),
            SystemDescriptor::new(|_| {}).reads::<RenderData>().named("render1"),
            SystemDescriptor::new(|_| {}).reads::<RenderData>().named("render2"),
            SystemDescriptor::new(|_| {}).writes::<RenderData>().named("render_write"),
        ];
        let groups = ParallelSchedule::build_groups(&systems);
        // physics + render1 + render2 can go together (disjoint or read-read)
        // render_write conflicts with render1 and render2 (write-read)
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 3); // physics, render1, render2
        assert_eq!(groups[1].len(), 1); // render_write
    }

    #[test]
    fn test_parallel_schedule_sequential_correctness() {
        // Verify that systems execute and produce correct results
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        fn sys_a(_: &mut World) {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }
        fn sys_b(_: &mut World) {
            COUNTER.fetch_add(10, Ordering::SeqCst);
        }
        fn sys_c(_: &mut World) {
            COUNTER.fetch_add(100, Ordering::SeqCst);
        }

        let mut schedule = ParallelSchedule::new()
            .with_stage("sim");
        schedule.add_system("sim", SystemDescriptor::new(sys_a).reads::<PhysicsData>());
        schedule.add_system("sim", SystemDescriptor::new(sys_b).reads::<RenderData>());
        schedule.add_system("sim", SystemDescriptor::new(sys_c).reads::<AudioData>());

        let mut world = World::new();
        schedule.run(&mut world);

        assert_eq!(COUNTER.load(Ordering::SeqCst), 111);
    }

    #[test]
    fn test_exclusive_system_runs_alone() {
        let systems = vec![
            SystemDescriptor::new(|_| {}).exclusive().named("exclusive"),
            SystemDescriptor::new(|_| {}).reads::<PhysicsData>().named("physics"),
        ];
        let groups = ParallelSchedule::build_groups(&systems);
        // Exclusive system must be in its own group
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[1].len(), 1);
    }

    #[test]
    fn test_undeclared_system_defaults_to_exclusive() {
        let desc = SystemDescriptor::new(|_| {});
        assert!(desc.access.exclusive, "undeclared access should default to exclusive");
    }

    #[test]
    fn test_descriptor_builder_clears_exclusive() {
        let desc = SystemDescriptor::new(|_| {}).reads::<PhysicsData>();
        assert!(!desc.access.exclusive, "reads() should clear exclusive flag");
    }
}
