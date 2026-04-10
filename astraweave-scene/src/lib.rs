#![forbid(unsafe_code)]
//! # AstraWeave Scene
//!
//! Hierarchical scene graph with transform management and world partitioning.
//!
//! This crate provides:
//!
//! - **[`Transform`]** — Translation/rotation/scale with matrix conversion,
//!   interpolation, composition, and inverse.
//! - **[`Node`]** — Scene graph node with transform, children, and mesh/material references.
//! - **[`Scene`]** — Top-level scene container managing the node hierarchy.
//! - **[`world_partition`]** — Spatial world partitioning for large open worlds.
//! - **[`streaming`]** — Async cell streaming with distance-based loading.
//! - **[`gpu_resource_manager`]** — GPU resource lifecycle management.
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `ecs` | ECS system integration types |
//! | `world-partition` | Spatial partitioning and streaming |

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

pub mod error;
pub mod gpu_resource_manager;
pub mod partitioned_scene;
pub mod streaming;
pub mod world_partition;

pub use error::{SceneError, SceneResult};

#[cfg(test)]
mod mutation_tests;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Creates a new transform with the given translation, rotation, and scale.
    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Creates an identity transform.
    #[inline]
    pub fn identity() -> Self {
        Self::default()
    }

    /// Creates a transform with only translation.
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    /// Creates a transform with only rotation.
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Default::default()
        }
    }

    /// Creates a transform with only uniform scale.
    pub fn from_scale(scale: f32) -> Self {
        Self {
            scale: Vec3::splat(scale),
            ..Default::default()
        }
    }

    /// Creates a transform with non-uniform scale.
    pub fn from_scale_vec(scale: Vec3) -> Self {
        Self {
            scale,
            ..Default::default()
        }
    }

    /// Returns the 4x4 transformation matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Returns true if this is an identity transform.
    pub fn is_identity(&self) -> bool {
        self.translation == Vec3::ZERO && self.rotation == Quat::IDENTITY && self.scale == Vec3::ONE
    }

    /// Returns true if the scale is uniform.
    pub fn is_uniform_scale(&self) -> bool {
        (self.scale.x - self.scale.y).abs() < f32::EPSILON
            && (self.scale.y - self.scale.z).abs() < f32::EPSILON
    }

    /// Returns the uniform scale (average of xyz).
    pub fn uniform_scale(&self) -> f32 {
        (self.scale.x + self.scale.y + self.scale.z) / 3.0
    }

    /// Returns the forward direction (negative Z in local space).
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// Returns the right direction (positive X in local space).
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Returns the up direction (positive Y in local space).
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Returns the inverse of this transform.
    pub fn inverse(&self) -> Self {
        let inv_rotation = self.rotation.inverse();
        let inv_scale = Vec3::new(1.0 / self.scale.x, 1.0 / self.scale.y, 1.0 / self.scale.z);
        let inv_translation = inv_rotation * (-self.translation * inv_scale);
        Self {
            translation: inv_translation,
            rotation: inv_rotation,
            scale: inv_scale,
        }
    }

    /// Transforms a point from local to world space.
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.rotation * (point * self.scale) + self.translation
    }

    /// Transforms a direction from local to world space (ignores translation).
    pub fn transform_direction(&self, direction: Vec3) -> Vec3 {
        self.rotation * direction
    }

    /// Linearly interpolates between two transforms.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Transform(pos={}, rot={}, scale={})",
            self.translation, self.rotation, self.scale
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub children: Vec<Node>,
}

impl Node {
    /// Creates a new node with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::default(),
            children: Vec::new(),
        }
    }

    /// Creates a new node with a transform.
    pub fn with_transform(name: impl Into<String>, transform: Transform) -> Self {
        Self {
            name: name.into(),
            transform,
            children: Vec::new(),
        }
    }

    /// Returns true if this node has children.
    #[inline]
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Returns the number of direct children.
    #[inline]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Returns the total number of descendants (recursive).
    pub fn descendant_count(&self) -> usize {
        self.children.iter().map(|c| 1 + c.descendant_count()).sum()
    }

    /// Returns true if this is a leaf node (no children).
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Adds a child node.
    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    /// Finds a child by name (non-recursive).
    pub fn find_child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Finds a child by name (non-recursive, mutable).
    pub fn find_child_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    /// Finds a descendant by name (recursive depth-first).
    pub fn find_descendant(&self, name: &str) -> Option<&Node> {
        for child in &self.children {
            if child.name == name {
                return Some(child);
            }
            if let Some(found) = child.find_descendant(name) {
                return Some(found);
            }
        }
        None
    }

    /// Returns the depth of this node's subtree.
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            0
        } else {
            1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.children.is_empty() {
            write!(f, "Node(\"{}\")", self.name)
        } else {
            write!(
                f,
                "Node(\"{}\", {} children)",
                self.name,
                self.children.len()
            )
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Scene {
    pub root: Node,
}

impl Scene {
    /// Creates a new scene with a default root node.
    pub fn new() -> Self {
        Self {
            root: Node::new("root"),
        }
    }

    /// Creates a scene with a custom root node.
    pub fn with_root(root: Node) -> Self {
        Self { root }
    }

    /// Returns the total number of nodes in the scene (including root).
    pub fn node_count(&self) -> usize {
        1 + self.root.descendant_count()
    }

    /// Returns true if the scene only has the root node.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.root.children.is_empty()
    }

    /// Returns the maximum depth of the scene graph.
    pub fn depth(&self) -> usize {
        self.root.depth()
    }

    /// Finds a node by name in the entire scene.
    pub fn find_node(&self, name: &str) -> Option<&Node> {
        if self.root.name == name {
            return Some(&self.root);
        }
        self.root.find_descendant(name)
    }

    /// Traverses the scene graph depth-first with world transforms.
    pub fn traverse<'a>(&'a self, f: &mut impl FnMut(&'a Node, Mat4)) {
        fn walk<'a>(n: &'a Node, parent: Mat4, f: &mut impl FnMut(&'a Node, Mat4)) {
            let world = parent * n.transform.matrix();
            f(n, world);
            for c in &n.children {
                walk(c, world, f);
            }
        }
        walk(&self.root, Mat4::IDENTITY, f);
    }

    /// Traverses with path tracking (node names from root).
    pub fn traverse_with_path<'a>(&'a self, f: &mut impl FnMut(&'a Node, Mat4, &[&str])) {
        fn walk<'a>(
            n: &'a Node,
            parent: Mat4,
            path: &mut Vec<&'a str>,
            f: &mut impl FnMut(&'a Node, Mat4, &[&str]),
        ) {
            let world = parent * n.transform.matrix();
            path.push(&n.name);
            f(n, world, path);
            for c in &n.children {
                walk(c, world, path, f);
            }
            path.pop();
        }
        let mut path = Vec::new();
        walk(&self.root, Mat4::IDENTITY, &mut path, f);
    }
}

impl std::fmt::Display for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Scene({} nodes, depth={})",
            self.node_count(),
            self.depth()
        )
    }
}

// ECS Components for scene graph integration
#[cfg(feature = "ecs")]
pub mod ecs {
    use super::*;
    use astraweave_ecs::{Entity as EntityId, World as EcsWorld};
    use std::collections::{BTreeMap, BTreeSet};

    /// Component for local transform (relative to parent)
    #[derive(Clone, Copy, Debug)]
    pub struct CTransformLocal(pub Transform);

    /// Component for world-space transform (computed from hierarchy)
    #[derive(Clone, Copy, Debug)]
    pub struct CTransformWorld(pub Mat4);

    /// Component for parent-child relationships
    #[derive(Clone, Debug)]
    pub struct CParent(pub EntityId);

    /// Component for children list (maintained automatically)
    #[derive(Clone, Debug, Default)]
    pub struct CChildren(pub Vec<EntityId>);

    /// Tag component indicating transform needs update
    #[derive(Clone, Copy, Debug)]
    pub struct CDirtyTransform;

    /// Resource tracking the last change-detection tick used by `mark_dirty_transforms`.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct ChangeDetectionTick(pub u32);

    /// Component for visibility culling
    #[derive(Clone, Copy, Debug)]
    pub struct CVisible(pub bool);

    /// Component for mesh handle (for rendering integration)
    #[derive(Clone, Copy, Debug)]
    pub struct CMesh(pub u32); // MeshHandle or index

    /// Component for material layer index (for rendering integration)
    #[derive(Clone, Copy, Debug)]
    pub struct CMaterial(pub u32); // Material layer index from MaterialManager

    /// Component for joint indices (skinning preparation)
    #[derive(Clone, Debug)]
    pub struct CJointIndices(pub Vec<u32>);

    // ========================================================================
    // Phase 2 Task 5: Skeletal Animation Components
    // ========================================================================

    /// Skeleton component (shared across instances via Arc)
    #[derive(Clone, Debug)]
    pub struct CSkeleton {
        pub joint_count: u32,
        pub root_indices: Vec<usize>,
        pub parent_indices: Vec<Option<usize>>,
        pub inverse_bind_matrices: Vec<Mat4>,
        pub local_transforms: Vec<Transform>, // Bind pose
    }

    /// Skinned mesh component
    #[derive(Clone, Debug)]
    pub struct CSkinnedMesh {
        pub mesh_handle: u32,
        pub max_influences: u8, // Typically 4
    }

    /// Animation playback state
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[non_exhaustive]
    pub enum PlaybackState {
        Playing,
        Paused,
        Stopped,
    }

    impl PlaybackState {
        /// Returns the name of this playback state.
        pub fn name(&self) -> &'static str {
            match self {
                Self::Playing => "Playing",
                Self::Paused => "Paused",
                Self::Stopped => "Stopped",
            }
        }

        /// Returns true if the animation is playing.
        #[inline]
        pub fn is_playing(&self) -> bool {
            matches!(self, Self::Playing)
        }

        /// Returns true if the animation is paused.
        #[inline]
        pub fn is_paused(&self) -> bool {
            matches!(self, Self::Paused)
        }

        /// Returns true if the animation is stopped.
        #[inline]
        pub fn is_stopped(&self) -> bool {
            matches!(self, Self::Stopped)
        }

        /// Returns true if the animation is active (playing or paused).
        #[inline]
        pub fn is_active(&self) -> bool {
            !self.is_stopped()
        }

        /// Returns all playback states.
        pub fn all() -> [PlaybackState; 3] {
            [Self::Playing, Self::Paused, Self::Stopped]
        }
    }

    impl std::fmt::Display for PlaybackState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    /// Animator component (per-entity animation state)
    #[derive(Clone, Debug, PartialEq)]
    pub struct CAnimator {
        pub clip_index: usize,
        pub time: f32,
        pub speed: f32,
        pub state: PlaybackState,
        pub looping: bool,
    }

    impl Default for CAnimator {
        fn default() -> Self {
            Self {
                clip_index: 0,
                time: 0.0,
                speed: 1.0,
                state: PlaybackState::Stopped,
                looping: true,
            }
        }
    }

    impl CAnimator {
        /// Creates a new animator for the given clip.
        pub fn new(clip_index: usize) -> Self {
            Self {
                clip_index,
                ..Default::default()
            }
        }

        /// Starts playing the animation.
        pub fn play(&mut self) {
            self.state = PlaybackState::Playing;
        }

        /// Pauses the animation.
        pub fn pause(&mut self) {
            if self.state == PlaybackState::Playing {
                self.state = PlaybackState::Paused;
            }
        }

        /// Stops the animation and resets to the beginning.
        pub fn stop(&mut self) {
            self.state = PlaybackState::Stopped;
            self.time = 0.0;
        }

        /// Toggles between playing and paused.
        pub fn toggle_pause(&mut self) {
            match self.state {
                PlaybackState::Playing => self.pause(),
                PlaybackState::Paused => self.play(),
                PlaybackState::Stopped => self.play(),
            }
        }

        /// Sets the animation speed.
        pub fn with_speed(mut self, speed: f32) -> Self {
            self.speed = speed;
            self
        }

        /// Sets whether the animation loops.
        pub fn with_looping(mut self, looping: bool) -> Self {
            self.looping = looping;
            self
        }

        /// Resets the animation time to the beginning.
        pub fn reset(&mut self) {
            self.time = 0.0;
        }

        /// Returns true if the animation is playing.
        #[inline]
        pub fn is_playing(&self) -> bool {
            self.state.is_playing()
        }

        /// Returns true if the animation is paused.
        #[inline]
        pub fn is_paused(&self) -> bool {
            self.state.is_paused()
        }

        /// Returns true if the animation is stopped.
        #[inline]
        pub fn is_stopped(&self) -> bool {
            self.state.is_stopped()
        }

        /// Returns the normalized time (0.0 to 1.0) given a clip duration.
        pub fn normalized_time(&self, duration: f32) -> f32 {
            if duration <= 0.0 {
                return 0.0;
            }
            (self.time / duration).clamp(0.0, 1.0)
        }
    }

    impl std::fmt::Display for CAnimator {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "CAnimator(clip={}, t={:.2}, speed={}, {}{})",
                self.clip_index,
                self.time,
                self.speed,
                self.state,
                if self.looping { ", looping" } else { "" }
            )
        }
    }

    /// Computed joint matrices (cached per-frame)
    #[derive(Clone, Debug)]
    pub struct CJointMatrices {
        pub matrices: Vec<Mat4>, // Skinning matrices (world * inverse_bind)
        pub dirty: bool,
    }

    impl Default for CJointMatrices {
        fn default() -> Self {
            Self {
                matrices: Vec::new(),
                dirty: true,
            }
        }
    }

    /// Tag component for entities with dirty animation state
    #[derive(Clone, Copy, Debug)]
    pub struct CDirtyAnimation;

    /// Component for attaching entity to a specific joint (bone attachment)
    #[derive(Clone, Copy, Debug)]
    pub struct CParentBone {
        pub skeleton_entity: EntityId,
        pub joint_index: usize,
    }

    /// Helper structure for managing scene graph operations
    pub struct SceneGraph;

    impl SceneGraph {
        /// Attach a child entity to a parent, updating parent's CChildren
        pub fn attach(world: &mut EcsWorld, child: EntityId, parent: EntityId) {
            // Set parent
            world.insert(child, CParent(parent));

            // Mark child as dirty
            world.insert(child, CDirtyTransform);

            // Update parent's children list
            if let Some(children) = world.get_mut::<CChildren>(parent) {
                if !children.0.contains(&child) {
                    children.0.push(child);
                }
            } else {
                world.insert(parent, CChildren(vec![child]));
            }
        }

        /// Detach a child from its parent
        pub fn detach(world: &mut EcsWorld, child: EntityId) {
            if let Some(parent_comp) = world.get::<CParent>(child) {
                let parent = parent_comp.0;

                // Remove from parent's children list
                if let Some(children) = world.get_mut::<CChildren>(parent) {
                    children.0.retain(|&c| c != child);
                }

                // Remove parent component
                world.remove::<CParent>(child);

                // Mark as dirty (now a root)
                world.insert(child, CDirtyTransform);
            }
        }

        /// Reparent a child to a new parent
        pub fn reparent(world: &mut EcsWorld, child: EntityId, new_parent: EntityId) {
            Self::detach(world, child);
            Self::attach(world, child, new_parent);
        }

        /// Mark an entity and all descendants as dirty
        #[allow(dead_code)]
        fn mark_dirty_recursive(
            world: &mut EcsWorld,
            entity: EntityId,
            visited: &mut BTreeSet<EntityId>,
        ) {
            if visited.contains(&entity) {
                return; // Avoid cycles
            }
            visited.insert(entity);

            world.insert(entity, CDirtyTransform);

            // Clone children list to avoid borrow checker issues
            let children = world.get::<CChildren>(entity).map(|c| c.0.clone());

            if let Some(children_list) = children {
                for child in children_list {
                    Self::mark_dirty_recursive(world, child, visited);
                }
            }
        }
    }

    /// System to mark dirty transforms when local transforms change.
    ///
    /// Uses ECS change detection: iterates entities whose `CTransformLocal`
    /// was modified since the last tick recorded in the `ChangeDetectionTick`
    /// resource, and inserts `CDirtyTransform` on each. After scanning, the
    /// tick is advanced so the next frame only sees new mutations.
    pub fn mark_dirty_transforms(world: &mut EcsWorld) {
        // Read the last-seen tick (default 0 on first frame)
        let last_tick = world
            .get_resource::<ChangeDetectionTick>()
            .map(|r| r.0)
            .unwrap_or(0);

        // Collect entities whose CTransformLocal changed since last_tick
        let mut changed: Vec<EntityId> = Vec::new();
        world.each_changed::<CTransformLocal>(last_tick, |entity, _| {
            changed.push(entity);
        });

        // Insert CDirtyTransform tag on each changed entity
        for entity in changed {
            if !world.has::<CDirtyTransform>(entity) {
                world.insert(entity, CDirtyTransform);
            }
        }

        // Advance the tick so next frame only picks up new changes
        let current_tick = world.change_tick();
        world.insert_resource(ChangeDetectionTick(current_tick));
    }

    /// System to update world transforms from hierarchy (deterministic topological order).
    ///
    /// Uses a three-phase bulk read → compute → write pattern to avoid
    /// scattered per-entity ECS access during the hot loop (3-5× faster
    /// than interleaved get/insert per entity).
    pub fn update_world_transforms(world: &mut EcsWorld) {
        // Phase 0: Early-out if nothing is dirty
        let mut dirty_count = 0u32;
        world.each_mut::<CDirtyTransform>(|_e, _| dirty_count += 1);
        if dirty_count == 0 {
            // Check if any entity still needs an initial world transform
            let mut has_local = false;
            let mut has_world = false;
            world.each_mut::<CTransformLocal>(|_e, _| has_local = true);
            world.each_mut::<CTransformWorld>(|_e, _| has_world = true);
            if !has_local || has_world {
                return;
            }
        }

        // Phase 1: Bulk-read all component data (cache-friendly each_mut iteration)
        let mut locals: Vec<(EntityId, Transform)> = Vec::new();
        world.each_mut::<CTransformLocal>(|e, local| {
            locals.push((e, local.0));
        });

        let mut parent_map: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        let mut children_map: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
        world.each_mut::<CParent>(|e, parent| {
            parent_map.insert(e, parent.0);
            children_map.entry(parent.0).or_default().push(e);
        });

        // Phase 2: Pure-CPU computation (no ECS access)
        let local_map: BTreeMap<EntityId, Transform> = locals.iter().copied().collect();

        // Find roots (entities with CTransformLocal but no CParent)
        let mut roots: Vec<EntityId> = locals
            .iter()
            .filter(|(e, _)| !parent_map.contains_key(e))
            .map(|(e, _)| *e)
            .collect();
        roots.sort_by_key(|e| e.id());

        // Sort children lists once for deterministic traversal
        for children in children_map.values_mut() {
            children.sort_by_key(|e| e.id());
        }

        // Depth-first traversal computing world matrices into a flat buffer
        let mut computed: Vec<(EntityId, Mat4)> = Vec::with_capacity(locals.len());

        fn compute_recursive(
            entity: EntityId,
            parent_world: Mat4,
            local_map: &BTreeMap<EntityId, Transform>,
            children_map: &BTreeMap<EntityId, Vec<EntityId>>,
            computed: &mut Vec<(EntityId, Mat4)>,
        ) {
            let local_mat = local_map
                .get(&entity)
                .map(|t| t.matrix())
                .unwrap_or(Mat4::IDENTITY);
            let world_mat = parent_world * local_mat;
            computed.push((entity, world_mat));

            if let Some(children) = children_map.get(&entity) {
                for &child in children {
                    compute_recursive(child, world_mat, local_map, children_map, computed);
                }
            }
        }

        for root in roots {
            compute_recursive(
                root,
                Mat4::IDENTITY,
                &local_map,
                &children_map,
                &mut computed,
            );
        }

        // Phase 3: Batch-write all results back to ECS
        for (entity, world_mat) in &computed {
            world.insert(*entity, CTransformWorld(*world_mat));
        }

        // Phase 4: Batch-clear dirty flags
        let mut dirty_entities: Vec<EntityId> = Vec::new();
        world.each_mut::<CDirtyTransform>(|e, _| dirty_entities.push(e));
        for entity in dirty_entities {
            world.remove::<CDirtyTransform>(entity);
        }
    }

    /// Render instance data (minimal, for renderer integration)
    #[derive(Clone, Debug)]
    pub struct RenderInstance {
        pub entity: EntityId,
        pub world_transform: Mat4,
        pub mesh_handle: u32,
        pub material_index: u32,
    }

    /// System to sync scene to renderer (collect visible instances)
    pub fn sync_scene_to_renderer(world: &mut EcsWorld) -> Vec<RenderInstance> {
        let mut instances = Vec::new();

        // First, collect all entities with world transforms
        let mut entities_with_transforms = Vec::new();
        world.each_mut::<CTransformWorld>(|e, _| {
            entities_with_transforms.push(e);
        });

        // Then query each entity for its components
        for entity in entities_with_transforms {
            // Check visibility
            let visible = world.get::<CVisible>(entity).map(|v| v.0).unwrap_or(true); // Default to visible

            if !visible {
                continue;
            }

            // Must have mesh and material
            let mesh_handle = match world.get::<CMesh>(entity) {
                Some(m) => m.0,
                None => continue,
            };

            let material_index = match world.get::<CMaterial>(entity) {
                Some(m) => m.0,
                None => continue,
            };

            let world_transform = world
                .get::<CTransformWorld>(entity)
                .map(|t| t.0)
                .unwrap_or(Mat4::IDENTITY);

            instances.push(RenderInstance {
                entity,
                world_transform,
                mesh_handle,
                material_index,
            });
        }

        // Sort for deterministic rendering order
        instances.sort_by_key(|inst| inst.entity.id());

        instances
    }

    // ========================================================================
    // Phase 2 Task 5: Skeletal Animation Systems
    // ========================================================================

    /// System to update animation time (advance playback)
    /// Call this each frame with delta time
    pub fn update_animations(world: &mut EcsWorld, dt: f32, clip_durations: &[f32]) {
        // Collect entities with animators
        let mut entities = Vec::new();
        world.each_mut::<CAnimator>(|e, _| entities.push(e));

        for entity in entities {
            if let Some(animator) = world.get_mut::<CAnimator>(entity) {
                if animator.state != PlaybackState::Playing {
                    continue;
                }

                let clip_duration = clip_durations
                    .get(animator.clip_index)
                    .copied()
                    .unwrap_or(1.0);

                // Advance time
                animator.time += dt * animator.speed;

                // Handle looping/clamping
                if animator.looping {
                    // Wrap around
                    if animator.time > clip_duration {
                        animator.time %= clip_duration;
                    }
                    if animator.time < 0.0 {
                        animator.time = clip_duration + (animator.time % clip_duration);
                    }
                } else {
                    // Clamp and stop at end
                    animator.time = animator.time.clamp(0.0, clip_duration);
                    if animator.time >= clip_duration {
                        animator.state = PlaybackState::Stopped;
                    }
                }

                // Mark for recomputation
                world.insert(entity, CDirtyAnimation);
            }
        }
    }

    /// System to compute joint matrices from animation state
    /// This uses the animation sampling and joint matrix computation from astraweave-render
    /// Note: This is a stub - full implementation requires AnimationClip from render crate
    pub fn compute_poses_stub(world: &mut EcsWorld) {
        // Collect entities needing pose update
        let mut entities = Vec::new();
        world.each_mut::<CDirtyAnimation>(|e, _| entities.push(e));

        for entity in entities {
            // Check if entity has all required components
            let has_skeleton = world.get::<CSkeleton>(entity).is_some();
            let has_matrices = world.get::<CJointMatrices>(entity).is_some();

            if !has_skeleton || !has_matrices {
                continue;
            }

            // Get skeleton to know joint count
            let joint_count = world
                .get::<CSkeleton>(entity)
                .map(|s| s.joint_count)
                .unwrap_or(0);

            // Mark matrices as dirty (actual computation happens in renderer integration)
            if let Some(matrices) = world.get_mut::<CJointMatrices>(entity) {
                matrices.dirty = true;
                // Ensure matrices vector has correct size
                if matrices.matrices.len() != joint_count as usize {
                    matrices
                        .matrices
                        .resize(joint_count as usize, Mat4::IDENTITY);
                }
            }

            // Remove dirty animation flag
            world.remove::<CDirtyAnimation>(entity);
        }
    }

    /// System to sync bone transforms to scene graph nodes
    /// This propagates joint world transforms to attached entities (e.g., weapon to hand)
    pub fn sync_bone_attachments(world: &mut EcsWorld) {
        // Collect all entities with bone attachments
        let mut attachments = Vec::new();
        world.each_mut::<CParentBone>(|e, _| attachments.push(e));

        for entity in attachments {
            if let Some(attachment) = world.get::<CParentBone>(entity) {
                let skeleton_entity = attachment.skeleton_entity;
                let joint_index = attachment.joint_index;

                // Get joint matrices from skeleton entity
                if let Some(matrices) = world.get::<CJointMatrices>(skeleton_entity) {
                    if joint_index < matrices.matrices.len() {
                        let joint_world_matrix = matrices.matrices[joint_index];

                        // Set attached entity's world transform to joint transform
                        world.insert(entity, CTransformWorld(joint_world_matrix));

                        // Also update local transform if entity has parent
                        // (for proper scene graph integration)
                        if let Some(parent_comp) = world.get::<CParent>(entity) {
                            let parent_world = world
                                .get::<CTransformWorld>(parent_comp.0)
                                .map(|t| t.0)
                                .unwrap_or(Mat4::IDENTITY);

                            // Compute local = parent_inv * world
                            // Note: glam 0.30 inverse() returns Mat4, not Option
                            let parent_inv = parent_world.inverse();
                            let local_mat = parent_inv * joint_world_matrix;
                            // Decompose to TRS (approximate for now)
                            let (scale, rotation, translation) =
                                local_mat.to_scale_rotation_translation();
                            world.insert(
                                entity,
                                CTransformLocal(Transform {
                                    translation,
                                    rotation,
                                    scale,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "ecs"))]
pub mod ecs {
    // Stub when ECS not enabled
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Quat, Vec3};
    use std::f32::consts::PI;

    // ===== Transform Tests =====

    #[test]
    fn test_transform_default() {
        let t = Transform::default();
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_transform_matrix_identity() {
        let t = Transform::default();
        let m = t.matrix();

        // Identity transform should produce identity matrix
        assert!((m - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 0.0001));
    }

    #[test]
    fn test_transform_matrix_translation() {
        let t = Transform {
            translation: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let m = t.matrix();

        // Extract translation from matrix
        let (_, _, extracted_translation) = m.to_scale_rotation_translation();
        assert!((extracted_translation - t.translation).length() < 0.0001);
    }

    #[test]
    fn test_transform_matrix_rotation() {
        let rotation = Quat::from_rotation_y(PI / 2.0); // 90 degrees around Y
        let t = Transform {
            translation: Vec3::ZERO,
            rotation,
            scale: Vec3::ONE,
        };
        let m = t.matrix();

        // Extract rotation from matrix
        let (_, extracted_rotation, _) = m.to_scale_rotation_translation();

        // Quaternions can have equivalent forms (q and -q represent same rotation)
        let dot = extracted_rotation.dot(rotation).abs();
        assert!(dot > 0.9999, "Rotations should be equivalent");
    }

    #[test]
    fn test_transform_matrix_scale() {
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::new(2.0, 3.0, 4.0),
        };
        let m = t.matrix();

        // Extract scale from matrix
        let (extracted_scale, _, _) = m.to_scale_rotation_translation();
        assert!((extracted_scale - t.scale).length() < 0.0001);
    }

    #[test]
    fn test_transform_matrix_combined() {
        let t = Transform {
            translation: Vec3::new(5.0, 10.0, 15.0),
            rotation: Quat::from_rotation_z(PI / 4.0), // 45 degrees around Z
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let m = t.matrix();

        // Transform a point
        let point = Vec3::new(1.0, 0.0, 0.0);
        let transformed = m.transform_point3(point);

        // Expected: scale (2.0), rotate 45° around Z, translate
        // Scaled: (2.0, 0.0, 0.0)
        // Rotated 45°: (sqrt(2), sqrt(2), 0)
        // Translated: (5 + sqrt(2), 10 + sqrt(2), 15)
        let sqrt2 = 2.0_f32.sqrt();
        let expected = Vec3::new(5.0 + sqrt2, 10.0 + sqrt2, 15.0);
        assert!((transformed - expected).length() < 0.0001);
    }

    #[test]
    fn test_transform_serialization() {
        let t = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_x(PI / 6.0),
            scale: Vec3::new(1.5, 2.5, 3.5),
        };

        let json = serde_json::to_string(&t).unwrap();
        let deserialized: Transform = serde_json::from_str(&json).unwrap();

        assert!((deserialized.translation - t.translation).length() < 0.0001);
        assert!((deserialized.scale - t.scale).length() < 0.0001);
    }

    // ===== Node Tests =====

    #[test]
    fn test_node_new() {
        let node = Node::new("test_node");
        assert_eq!(node.name, "test_node");
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_node_new_string() {
        let node = Node::new(String::from("string_node"));
        assert_eq!(node.name, "string_node");
    }

    #[test]
    fn test_node_default() {
        let node = Node::default();
        assert!(node.name.is_empty());
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_node_with_children() {
        let mut parent = Node::new("parent");
        parent.children.push(Node::new("child1"));
        parent.children.push(Node::new("child2"));

        assert_eq!(parent.children.len(), 2);
        assert_eq!(parent.children[0].name, "child1");
        assert_eq!(parent.children[1].name, "child2");
    }

    #[test]
    fn test_node_with_transform() {
        let mut node = Node::new("transformed");
        node.transform.translation = Vec3::new(10.0, 0.0, 0.0);
        node.transform.scale = Vec3::new(2.0, 2.0, 2.0);

        assert_eq!(node.transform.translation, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(node.transform.scale, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_node_serialization() {
        let mut node = Node::new("serialized");
        node.transform.translation = Vec3::new(1.0, 2.0, 3.0);
        node.children.push(Node::new("child"));

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: Node = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "serialized");
        assert_eq!(deserialized.children.len(), 1);
        assert_eq!(deserialized.children[0].name, "child");
    }

    // ===== Scene Tests =====

    #[test]
    fn test_scene_new() {
        let scene = Scene::new();
        assert_eq!(scene.root.name, "root");
        assert!(scene.root.children.is_empty());
    }

    #[test]
    fn test_scene_default() {
        let scene = Scene::default();
        assert!(scene.root.name.is_empty());
    }

    #[test]
    fn test_scene_traverse_single_node() {
        let scene = Scene::new();
        let mut count = 0;

        scene.traverse(&mut |node, _matrix| {
            count += 1;
            assert_eq!(node.name, "root");
        });

        assert_eq!(count, 1);
    }

    #[test]
    fn test_scene_traverse_hierarchy() {
        let mut scene = Scene::new();
        scene.root.children.push(Node::new("child1"));
        scene.root.children.push(Node::new("child2"));
        scene.root.children[0]
            .children
            .push(Node::new("grandchild"));

        let mut names = Vec::new();
        scene.traverse(&mut |node, _matrix| {
            names.push(node.name.clone());
        });

        // Should visit all 4 nodes: root, child1, grandchild, child2
        assert_eq!(names.len(), 4);
        assert_eq!(names[0], "root");
        assert!(names.contains(&"child1".to_string()));
        assert!(names.contains(&"child2".to_string()));
        assert!(names.contains(&"grandchild".to_string()));
    }

    #[test]
    fn test_scene_traverse_world_matrix_propagation() {
        let mut scene = Scene::new();
        scene.root.transform.translation = Vec3::new(10.0, 0.0, 0.0);

        let mut child = Node::new("child");
        child.transform.translation = Vec3::new(5.0, 0.0, 0.0);
        scene.root.children.push(child);

        let mut world_translations = Vec::new();
        scene.traverse(&mut |node, matrix| {
            let (_, _, translation) = matrix.to_scale_rotation_translation();
            world_translations.push((node.name.clone(), translation));
        });

        // Root should be at (10, 0, 0)
        // Child should be at (15, 0, 0) = parent + local
        assert_eq!(world_translations.len(), 2);

        let root_trans = world_translations
            .iter()
            .find(|(n, _)| n == "root")
            .unwrap()
            .1;
        let child_trans = world_translations
            .iter()
            .find(|(n, _)| n == "child")
            .unwrap()
            .1;

        assert!((root_trans - Vec3::new(10.0, 0.0, 0.0)).length() < 0.0001);
        assert!((child_trans - Vec3::new(15.0, 0.0, 0.0)).length() < 0.0001);
    }

    #[test]
    fn test_scene_traverse_scale_inheritance() {
        let mut scene = Scene::new();
        scene.root.transform.scale = Vec3::new(2.0, 2.0, 2.0);

        let mut child = Node::new("child");
        child.transform.scale = Vec3::new(2.0, 2.0, 2.0);
        scene.root.children.push(child);

        let mut world_scales = Vec::new();
        scene.traverse(&mut |node, matrix| {
            let (scale, _, _) = matrix.to_scale_rotation_translation();
            world_scales.push((node.name.clone(), scale));
        });

        // Root scale: 2.0
        // Child world scale: 2.0 * 2.0 = 4.0
        let root_scale = world_scales.iter().find(|(n, _)| n == "root").unwrap().1;
        let child_scale = world_scales.iter().find(|(n, _)| n == "child").unwrap().1;

        assert!((root_scale - Vec3::new(2.0, 2.0, 2.0)).length() < 0.0001);
        assert!((child_scale - Vec3::new(4.0, 4.0, 4.0)).length() < 0.0001);
    }

    #[test]
    fn test_scene_serialization() {
        let mut scene = Scene::new();
        scene.root.transform.translation = Vec3::new(1.0, 2.0, 3.0);
        scene.root.children.push(Node::new("child"));

        let json = serde_json::to_string(&scene).unwrap();
        let deserialized: Scene = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.root.children.len(), 1);
    }

    #[test]
    fn test_scene_deep_hierarchy() {
        let mut scene = Scene::new();

        // Create a deep hierarchy: root -> level1 -> level2 -> level3 -> level4
        let mut current = &mut scene.root;
        for i in 1..=4 {
            current.children.push(Node::new(format!("level{}", i)));
            current = &mut current.children[0];
        }

        let mut depth = 0;
        scene.traverse(&mut |_node, _matrix| {
            depth += 1;
        });

        assert_eq!(depth, 5); // root + 4 levels
    }

    #[test]
    fn test_scene_traverse_rotation_propagation() {
        let mut scene = Scene::new();
        scene.root.transform.rotation = Quat::from_rotation_y(PI / 2.0); // 90° around Y

        let mut child = Node::new("child");
        child.transform.translation = Vec3::new(10.0, 0.0, 0.0); // 10 units in X
        scene.root.children.push(child);

        let mut child_world_pos = Vec3::ZERO;
        scene.traverse(&mut |node, matrix| {
            if node.name == "child" {
                let (_, _, translation) = matrix.to_scale_rotation_translation();
                child_world_pos = translation;
            }
        });

        // After 90° rotation around Y, X becomes Z
        // Child at local (10, 0, 0) should be at world (0, 0, -10)
        assert!((child_world_pos - Vec3::new(0.0, 0.0, -10.0)).length() < 0.001);
    }

    // ===== New Transform Helper Tests =====

    #[test]
    fn test_transform_new() {
        let t = Transform::new(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_x(0.5),
            Vec3::new(2.0, 2.0, 2.0),
        );
        assert_eq!(t.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.scale, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_transform_identity() {
        let t = Transform::identity();
        assert!(t.is_identity());
    }

    #[test]
    fn test_transform_from_translation() {
        let t = Transform::from_translation(Vec3::new(5.0, 10.0, 15.0));
        assert_eq!(t.translation, Vec3::new(5.0, 10.0, 15.0));
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_transform_from_rotation() {
        let rot = Quat::from_rotation_y(PI / 2.0);
        let t = Transform::from_rotation(rot);
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_transform_from_scale() {
        let t = Transform::from_scale(3.0);
        assert_eq!(t.scale, Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn test_transform_from_scale_vec() {
        let t = Transform::from_scale_vec(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.scale, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_transform_is_identity() {
        assert!(Transform::default().is_identity());
        assert!(!Transform::from_translation(Vec3::X).is_identity());
    }

    #[test]
    fn test_transform_is_uniform_scale() {
        assert!(Transform::from_scale(2.0).is_uniform_scale());
        assert!(!Transform::from_scale_vec(Vec3::new(1.0, 2.0, 3.0)).is_uniform_scale());
    }

    #[test]
    fn test_transform_uniform_scale() {
        let t = Transform::from_scale_vec(Vec3::new(3.0, 6.0, 9.0));
        assert!((t.uniform_scale() - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_directions() {
        let t = Transform::identity();
        assert!((t.forward() - Vec3::NEG_Z).length() < 0.001);
        assert!((t.right() - Vec3::X).length() < 0.001);
        assert!((t.up() - Vec3::Y).length() < 0.001);
    }

    #[test]
    fn test_transform_transform_point() {
        let t = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let p = t.transform_point(Vec3::ZERO);
        assert!((p - Vec3::new(10.0, 0.0, 0.0)).length() < 0.001);
    }

    #[test]
    fn test_transform_transform_direction() {
        let t = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let d = t.transform_direction(Vec3::X);
        // Direction should ignore translation
        assert!((d - Vec3::X).length() < 0.001);
    }

    #[test]
    fn test_transform_lerp() {
        let a = Transform::from_translation(Vec3::ZERO);
        let b = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let mid = a.lerp(&b, 0.5);
        assert!((mid.translation - Vec3::new(5.0, 0.0, 0.0)).length() < 0.001);
    }

    #[test]
    fn test_transform_display() {
        let t = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let s = format!("{}", t);
        assert!(s.contains("Transform"));
        assert!(s.contains("pos="));
    }

    // ===== New Node Helper Tests =====

    #[test]
    fn test_node_with_transform_constructor() {
        let t = Transform::from_translation(Vec3::X);
        let node = Node::with_transform("moved", t);
        assert_eq!(node.name, "moved");
        assert_eq!(node.transform.translation, Vec3::X);
    }

    #[test]
    fn test_node_has_children() {
        let mut node = Node::new("parent");
        assert!(!node.has_children());
        node.add_child(Node::new("child"));
        assert!(node.has_children());
    }

    #[test]
    fn test_node_child_count() {
        let mut node = Node::new("parent");
        assert_eq!(node.child_count(), 0);
        node.add_child(Node::new("child1"));
        node.add_child(Node::new("child2"));
        assert_eq!(node.child_count(), 2);
    }

    #[test]
    fn test_node_descendant_count() {
        let mut root = Node::new("root");
        let mut child = Node::new("child");
        child.add_child(Node::new("grandchild"));
        root.add_child(child);
        root.add_child(Node::new("child2"));
        // 2 children + 1 grandchild = 3 descendants
        assert_eq!(root.descendant_count(), 3);
    }

    #[test]
    fn test_node_is_leaf() {
        let mut node = Node::new("test");
        assert!(node.is_leaf());
        node.add_child(Node::new("child"));
        assert!(!node.is_leaf());
    }

    #[test]
    fn test_node_find_child() {
        let mut node = Node::new("parent");
        node.add_child(Node::new("child1"));
        node.add_child(Node::new("child2"));
        assert!(node.find_child("child1").is_some());
        assert!(node.find_child("nonexistent").is_none());
    }

    #[test]
    fn test_node_find_child_mut() {
        let mut node = Node::new("parent");
        node.add_child(Node::new("child1"));
        if let Some(child) = node.find_child_mut("child1") {
            child.name = "renamed".to_string();
        }
        assert!(node.find_child("renamed").is_some());
    }

    #[test]
    fn test_node_find_descendant() {
        let mut root = Node::new("root");
        let mut child = Node::new("child");
        child.add_child(Node::new("grandchild"));
        root.add_child(child);
        assert!(root.find_descendant("grandchild").is_some());
        assert!(root.find_descendant("nonexistent").is_none());
    }

    #[test]
    fn test_node_depth() {
        let mut root = Node::new("root");
        assert_eq!(root.depth(), 0);
        let mut child = Node::new("child");
        child.add_child(Node::new("grandchild"));
        root.add_child(child);
        assert_eq!(root.depth(), 2);
    }

    #[test]
    fn test_node_display() {
        let node = Node::new("test");
        assert!(format!("{}", node).contains("Node(\"test\")"));

        let mut parent = Node::new("parent");
        parent.add_child(Node::new("child"));
        assert!(format!("{}", parent).contains("1 children"));
    }

    // ===== New Scene Helper Tests =====

    #[test]
    fn test_scene_with_root() {
        let root = Node::new("custom_root");
        let scene = Scene::with_root(root);
        assert_eq!(scene.root.name, "custom_root");
    }

    #[test]
    fn test_scene_node_count() {
        let mut scene = Scene::new();
        assert_eq!(scene.node_count(), 1); // Just root
        scene.root.add_child(Node::new("child1"));
        scene.root.add_child(Node::new("child2"));
        assert_eq!(scene.node_count(), 3);
    }

    #[test]
    fn test_scene_is_empty() {
        let scene = Scene::new();
        assert!(scene.is_empty());
        let mut non_empty = Scene::new();
        non_empty.root.add_child(Node::new("child"));
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_scene_depth() {
        let mut scene = Scene::new();
        assert_eq!(scene.depth(), 0);
        let mut child = Node::new("child");
        child.add_child(Node::new("grandchild"));
        scene.root.add_child(child);
        assert_eq!(scene.depth(), 2);
    }

    #[test]
    fn test_scene_find_node() {
        let mut scene = Scene::new();
        scene.root.add_child(Node::new("child"));
        assert!(scene.find_node("root").is_some());
        assert!(scene.find_node("child").is_some());
        assert!(scene.find_node("nonexistent").is_none());
    }

    #[test]
    fn test_scene_traverse_with_path() {
        let mut scene = Scene::new();
        scene.root.add_child(Node::new("child"));

        let mut paths: Vec<String> = Vec::new();
        scene.traverse_with_path(&mut |_node, _mat, path| {
            paths.push(path.join("/"));
        });
        assert!(paths.contains(&"root".to_string()));
        assert!(paths.contains(&"root/child".to_string()));
    }

    #[test]
    fn test_scene_display() {
        let scene = Scene::new();
        let s = format!("{}", scene);
        assert!(s.contains("Scene("));
        assert!(s.contains("nodes"));
    }
}
