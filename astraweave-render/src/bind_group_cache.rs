//! Bind group cache to eliminate per-frame `device.create_bind_group()` overhead.
//!
//! Many render passes create identical bind groups every frame because their
//! input textures and buffers haven't changed. This module provides a
//! generation-based cache: bind groups are rebuilt only when the renderer
//! signals a resource change (e.g., resize, texture swap) by bumping the
//! generation counter.
//!
//! # Usage
//! ```ignore
//! // In pass struct:
//! cached_bg: CachedBindGroup,
//!
//! // In execute():
//! let bg = self.cached_bg.get_or_rebuild(current_gen, || {
//!     device.create_bind_group(&descriptor)
//! });
//! pass.set_bind_group(0, bg, &[]);
//! ```

/// Monotonic generation counter. Bumped by the renderer on resize or
/// other resource-invalidating events.
pub type Generation = u64;

/// A lazily-cached bind group that rebuilds when the generation advances.
pub struct CachedBindGroup {
    bind_group: Option<wgpu::BindGroup>,
    generation: Generation,
}

impl CachedBindGroup {
    /// Create an empty (uncached) entry.
    pub fn new() -> Self {
        Self {
            bind_group: None,
            generation: 0,
        }
    }

    /// Create a pre-populated cache entry at the given generation.
    pub fn with_bind_group(bg: wgpu::BindGroup, gen: Generation) -> Self {
        Self {
            bind_group: Some(bg),
            generation: gen,
        }
    }

    /// Get the cached bind group, or rebuild it if stale.
    ///
    /// `current_gen` is the renderer's current generation counter.
    /// `rebuild` is called only if the cache is empty or the generation changed.
    pub fn get_or_rebuild(
        &mut self,
        current_gen: Generation,
        rebuild: impl FnOnce() -> wgpu::BindGroup,
    ) -> &wgpu::BindGroup {
        if self.bind_group.is_none() || self.generation != current_gen {
            self.bind_group = Some(rebuild());
            self.generation = current_gen;
        }
        // SAFETY: we just ensured it's Some above.
        self.bind_group.as_ref().expect("just assigned")
    }

    /// Explicitly invalidate the cache, forcing a rebuild on next access.
    pub fn invalidate(&mut self) {
        self.bind_group = None;
    }

    /// Check whether the cache is populated and current.
    pub fn is_valid(&self, current_gen: Generation) -> bool {
        self.bind_group.is_some() && self.generation == current_gen
    }
}

impl Default for CachedBindGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// A set of cached bind groups for passes that need multiple groups per frame.
///
/// Each slot is independently versioned so partial rebuilds are possible
/// (e.g., one group depends on depth and another on HDR, which may be
/// invalidated at different times).
pub struct CachedBindGroupSet {
    groups: Vec<CachedBindGroup>,
}

impl CachedBindGroupSet {
    /// Create a set with `count` empty slots.
    pub fn new(count: usize) -> Self {
        Self {
            groups: (0..count).map(|_| CachedBindGroup::new()).collect(),
        }
    }

    /// Get or rebuild the bind group at `index`.
    pub fn get_or_rebuild(
        &mut self,
        index: usize,
        current_gen: Generation,
        rebuild: impl FnOnce() -> wgpu::BindGroup,
    ) -> &wgpu::BindGroup {
        self.groups[index].get_or_rebuild(current_gen, rebuild)
    }

    /// Invalidate all cached bind groups.
    pub fn invalidate_all(&mut self) {
        for bg in &mut self.groups {
            bg.invalidate();
        }
    }

    /// Check whether the bind group at `index` is populated and current.
    pub fn is_valid(&self, index: usize, current_gen: Generation) -> bool {
        self.groups[index].is_valid(current_gen)
    }

    /// Set the bind group at `index` to a pre-built bind group at the given generation.
    pub fn set(&mut self, index: usize, bg: wgpu::BindGroup, gen: Generation) {
        self.groups[index] = CachedBindGroup::with_bind_group(bg, gen);
    }

    /// Number of slots.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Whether all slots are empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}
